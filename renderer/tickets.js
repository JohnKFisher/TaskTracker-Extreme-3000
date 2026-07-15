let desk365Base = '';
const POLL_INTERVAL = 5 * 60 * 1000;
const MIN_FETCH_INTERVAL_MS = 2100;

let ticketState = null;
let hiddenTicketDocument = {
  schemaVersion: 2,
  revision: 0,
  updatedAt: null,
  updatedBy: null,
  tickets: [],
  states: [],
};
let currentTickets = [];
let seenTicketNumbers = new Set();
// Ticket numbers from the most recent "new ticket" notification, consumed the next
// time the app window regains focus (see jumpToPendingTicketNotification in app.js's
// onFocusChanged listener) to jump straight to them instead of leaving the user to
// go find the tickets tab themselves.
let pendingNotificationTicketNumbers = [];
let pollTimer = null;
let hiddenTicketSaveInFlight = false;
let ticketFetchInFlight = null;
let queuedFetchTimer = null;
let lastTicketFetchStartedAt = 0;
let initializeTicketsPromise = null;

const ticketsList = document.getElementById('tickets-list');
const ticketsStatus = document.getElementById('tickets-status');
const apiKeySetup = document.getElementById('api-key-setup');
const refreshBtn = document.getElementById('btn-refresh-tickets');
const showHiddenCheckbox = document.getElementById('show-hidden-tickets');
const changeApiKeyBtn = document.getElementById('btn-change-api-key');
const saveApiKeyBtn = document.getElementById('btn-save-api-key');
const newTicketBtn = document.getElementById('btn-new-ticket');

function stopPolling() {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = null;
}

function clearQueuedFetchTimer() {
  if (queuedFetchTimer) clearTimeout(queuedFetchTimer);
  queuedFetchTimer = null;
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(fetchAndRenderTickets, POLL_INTERVAL);
}

// The stale/recent tint on a ticket card is only computed when its DOM node is
// (re)created, which normally happens after each poll — but a failed/skipped fetch
// (Desk365 down, rate-limited, briefly unconfigured) shouldn't also leave a "recent"
// tint stuck well past its 2-hour window. Update existing nodes in place (no network
// call, no full rebuild) instead of forcing a renderTickets() on a timer.
function refreshTicketFreshnessIndicators() {
  document.querySelectorAll('.ticket-card').forEach((card) => {
    const numberEl = card.querySelector('.ticket-number');
    if (!numberEl) return;
    const ticketNumber = numberEl.textContent.replace(/^#/, '');
    const ticket = currentTickets.find((entry) => String(entry.TicketNumber) === ticketNumber);
    if (!ticket) return;
    if (window.businessDaysSince(ticket.UpdatedAt) >= 5) card.dataset.stale = 'true';
    else delete card.dataset.stale;
    if (window.hoursSince(ticket.CreatedAt) < 2) card.dataset.recent = 'true';
    else delete card.dataset.recent;
  });
}

setInterval(refreshTicketFreshnessIndicators, 5 * 60 * 1000);

function hiddenTicketNumbers() {
  return new Set(
    (hiddenTicketDocument.states || [])
      .filter((state) => state.hidden)
      .map((state) => state.ticketNumber),
  );
}

function setTicketSetupVisible(visible) {
  apiKeySetup.classList.toggle('hidden', !visible);
}

function setTicketControlsEnabled(enabled) {
  refreshBtn.disabled = !enabled;
  showHiddenCheckbox.disabled = !enabled;
  changeApiKeyBtn.disabled = !enabled;
  saveApiKeyBtn.disabled = !enabled;
  document.getElementById('domain-input').disabled = !enabled;
  document.getElementById('api-key-input').disabled = !enabled;
}

function setDesk365Base(domain) {
  desk365Base = domain ? `https://${domain}/app/tickets/ticketdetails?tktNum=` : '';
  newTicketBtn.classList.toggle('hidden', !domain);
}

newTicketBtn.addEventListener('click', () => {
  if (!ticketState?.desk365Domain) return;
  window.openExternal(`https://${ticketState.desk365Domain}/app/tickets/createticket`).catch(() => {});
});

function applyHiddenTicketDocument(document) {
  hiddenTicketDocument = {
    schemaVersion: document.schemaVersion || 2,
    revision: document.revision || 0,
    updatedAt: document.updatedAt || null,
    updatedBy: document.updatedBy || null,
    tickets: Array.isArray(document.tickets) ? document.tickets : [],
    states: Array.isArray(document.states) ? document.states : [],
  };
}

async function loadTicketState() {
  ticketState = await window.callCommand('load_ticket_settings');
  const hiddenData = await window.callCommand('load_hidden_tickets');
  applyHiddenTicketDocument(hiddenData);
  setDesk365Base(ticketState.desk365Domain || '');
  document.getElementById('domain-input').value = ticketState.desk365Domain || '';
}

function renderTickets() {
  const showHidden = showHiddenCheckbox.checked;
  const hiddenNumbers = hiddenTicketNumbers();
  ticketsList.innerHTML = '';

  const filtered = showHidden
    ? currentTickets
    : currentTickets.filter((ticket) => !hiddenNumbers.has(ticket.TicketNumber));

  if (!filtered.length) {
    ticketsList.innerHTML = '<div class="empty-state">No tickets to show</div>';
    window.syncCompact(ticketsList);
    return;
  }

  filtered.forEach((ticket) => {
    const isHidden = hiddenNumbers.has(ticket.TicketNumber);
    const card = document.createElement('div');
    card.className = `ticket-card${isHidden ? ' hidden-ticket' : ''}`;
    if (window.businessDaysSince(ticket.UpdatedAt) >= 5) card.dataset.stale = 'true';
    if (window.hoursSince(ticket.CreatedAt) < 2) card.dataset.recent = 'true';

    const info = document.createElement('button');
    info.className = 'ticket-info';
    info.type = 'button';
    info.addEventListener('click', () => {
      window.openExternal(`${desk365Base}${ticket.TicketNumber}`).catch((error) => {
        ticketsStatus.textContent = error.message || 'Could not open the Desk365 ticket.';
      });
    });

    const num = document.createElement('span');
    num.className = 'ticket-number';
    num.textContent = `#${ticket.TicketNumber}`;

    const subject = document.createElement('div');
    subject.className = 'ticket-subject';
    subject.textContent = ticket.Subject || '(no subject)';

    const meta = document.createElement('div');
    meta.className = 'ticket-meta';

    const status = document.createElement('span');
    status.className = `ticket-status ${(ticket.Status || '').toLowerCase().replace(/\s+/g, '')}`;
    status.textContent = ticket.Status || 'Unknown';
    meta.appendChild(status);

    if (ticket.Priority) {
      const priority = document.createElement('span');
      priority.textContent = ticket.Priority;
      meta.appendChild(priority);
    }

    if (typeof ticket.Agent === 'string' && ticket.Agent) {
      const agent = document.createElement('span');
      agent.textContent = ticket.Agent.split('@')[0];
      meta.appendChild(agent);
    }

    info.appendChild(num);
    info.appendChild(subject);
    info.appendChild(meta);

    const hideBtn = document.createElement('button');
    hideBtn.className = 'ticket-hide-btn';
    hideBtn.type = 'button';
    hideBtn.textContent = isHidden ? '👁' : '🚫';
    hideBtn.title = isHidden ? 'Unhide ticket' : 'Hide ticket';
    hideBtn.setAttribute('aria-label', `${isHidden ? 'Unhide' : 'Hide'} ticket ${ticket.TicketNumber}`);
    hideBtn.disabled = !(window.currentStorageStatus && window.currentStorageStatus.sharedDataAvailable) || hiddenTicketSaveInFlight;
    hideBtn.addEventListener('click', (event) => {
      event.stopPropagation();
      hideBtn.disabled = true;
      toggleHideTicket(ticket.TicketNumber);
    });

    card.appendChild(info);
    card.appendChild(hideBtn);
    ticketsList.appendChild(card);
  });
  window.syncCompact(ticketsList);
}

// The OS-native notification plugin this app uses has no on-click callback on
// desktop, so a click can't tell us "open ticket #X" directly. The best available
// proxy: whenever the app window next regains focus (tray click, global shortcut,
// or the OS bringing us forward after the user clicks the notification itself),
// jump to the Tickets tab and highlight whichever ticket(s) triggered the last
// notification. Called from app.js's onFocusChanged listener.
window.jumpToPendingTicketNotification = function jumpToPendingTicketNotification() {
  if (!pendingNotificationTicketNumbers.length) return;
  const targetNumbers = pendingNotificationTicketNumbers.map(String);
  pendingNotificationTicketNumbers = [];

  window.setActiveTab('tickets');

  setTimeout(() => {
    const card = Array.from(ticketsList.querySelectorAll('.ticket-card')).find((entry) => {
      const numberEl = entry.querySelector('.ticket-number');
      return numberEl && targetNumbers.includes(numberEl.textContent.replace(/^#/, ''));
    });
    if (!card) return;
    card.scrollIntoView({ behavior: 'smooth', block: 'center' });
    card.classList.add('ticket-highlight');
    setTimeout(() => card.classList.remove('ticket-highlight'), 2500);
  }, 50);
};

async function toggleHideTicket(ticketNumber) {
  if (!(window.currentStorageStatus && window.currentStorageStatus.sharedDataAvailable)) return;
  // The whole hidden-tickets document is saved as one unit, so a save already in
  // flight covers any click that lands before it resolves — avoid a redundant
  // duplicate save_hidden_tickets call racing the same read-modify-write.
  if (hiddenTicketSaveInFlight) return;

  const existing = Array.isArray(hiddenTicketDocument.states) ? [...hiddenTicketDocument.states] : [];
  const updatedAt = new Date().toISOString();
  const nextHidden = !hiddenTicketNumbers().has(ticketNumber);
  const filtered = existing.filter((state) => state.ticketNumber !== ticketNumber);
  filtered.push({
    ticketNumber: String(ticketNumber),
    hidden: nextHidden,
    updatedAt,
  });

  try {
    hiddenTicketSaveInFlight = true;
    const result = await window.callCommand('save_hidden_tickets', {
      document: {
        schemaVersion: hiddenTicketDocument.schemaVersion || 2,
        revision: hiddenTicketDocument.revision || 0,
        updatedAt,
        states: filtered,
      },
    });
    applyHiddenTicketDocument(result.document);
    renderTickets();
  } catch (error) {
    ticketsStatus.textContent = error.message || 'Could not save hidden ticket state.';
    try {
      const hiddenData = await window.callCommand('load_hidden_tickets');
      applyHiddenTicketDocument(hiddenData);
      renderTickets();
    } catch (reloadError) {
      console.error('Failed to restore hidden ticket state after save error:', reloadError);
    }
  } finally {
    hiddenTicketSaveInFlight = false;
  }
}

async function runTicketFetch(forceFull = false) {
  if (!(ticketState && ticketState.hasApiKey && ticketState.desk365Domain)) return;
  if (!(window.currentStorageStatus && window.currentStorageStatus.sharedDataAvailable)) return;

  ticketsStatus.textContent = 'Loading tickets…';
  refreshBtn.disabled = true;
  lastTicketFetchStartedAt = Date.now();

  try {
    const result = await window.callCommand('fetch_tickets', { forceFull });
    const newTickets = result.tickets || [];

    if (seenTicketNumbers.size > 0) {
      const brandNew = newTickets.filter((ticket) => !seenTicketNumbers.has(ticket.TicketNumber));
      if (brandNew.length > 0) {
        const summary = brandNew.length === 1
          ? `#${brandNew[0].TicketNumber}: ${brandNew[0].Subject}`
          : `${brandNew.length} new tickets`;
        pendingNotificationTicketNumbers = brandNew.map((ticket) => ticket.TicketNumber);
        await window.callCommand('show_notification', {
          title: 'New Desk365 Tickets',
          body: summary,
        });
      }
    }

    newTickets.sort((a, b) => (parseInt(b.TicketNumber, 10) || 0) - (parseInt(a.TicketNumber, 10) || 0));

    seenTicketNumbers = new Set(newTickets.map((ticket) => ticket.TicketNumber));
    currentTickets = newTickets;
    window.updateTabCount('tickets', currentTickets.length);
    ticketsStatus.textContent = `${newTickets.length} unresolved tickets (updated ${new Date().toLocaleTimeString()})`;
    renderTickets();
  } catch (error) {
    ticketsStatus.textContent = error.message || 'Could not load tickets.';
    setTicketSetupVisible(true);
  } finally {
    refreshBtn.disabled = !(window.currentStorageStatus && window.currentStorageStatus.sharedDataAvailable);
  }
}

function fetchAndRenderTickets(options = {}) {
  const { force = false, forceFull = false } = options;

  if (ticketFetchInFlight) {
    return ticketFetchInFlight;
  }

  const now = Date.now();
  const remainingMs = MIN_FETCH_INTERVAL_MS - (now - lastTicketFetchStartedAt);
  if (remainingMs > 0) {
    if (!force) {
      return Promise.resolve();
    }

    ticketsStatus.textContent = 'Waiting a moment to avoid Desk365 rate limiting…';
    clearQueuedFetchTimer();
    return new Promise((resolve, reject) => {
      queuedFetchTimer = setTimeout(() => {
        queuedFetchTimer = null;
        fetchAndRenderTickets({ force: true, forceFull }).then(resolve).catch(reject);
      }, remainingMs);
    });
  }

  ticketFetchInFlight = runTicketFetch(forceFull)
    .finally(() => {
      ticketFetchInFlight = null;
    });
  return ticketFetchInFlight;
}

async function performInitializeTickets() {
  stopPolling();

  try {
    await loadTicketState();
  } catch (error) {
    ticketsStatus.textContent = error.message || 'Could not load Desk365 settings.';
    ticketsList.innerHTML = '';
    setTicketSetupVisible(true);
    return;
  }

  const storageAvailable = Boolean(window.currentStorageStatus && window.currentStorageStatus.sharedDataAvailable);
  setTicketControlsEnabled(storageAvailable);

  if (ticketState.authError) {
    ticketsStatus.textContent = ticketState.authError;
    setTicketSetupVisible(true);
    ticketsList.innerHTML = '';
    return;
  }

  if (ticketState.desk365Domain && ticketState.hasApiKey && storageAvailable) {
    setTicketSetupVisible(false);
    await fetchAndRenderTickets({ force: true });
    startPolling();
  } else {
    currentTickets = [];
    seenTicketNumbers = new Set();
    setTicketSetupVisible(true);
    ticketsList.innerHTML = '';
    if (!storageAvailable) {
      ticketsStatus.textContent = window.currentStorageStatus.message || 'Desk365 settings are unavailable while shared data is offline.';
    } else if (ticketState.desk365Domain && !ticketState.hasApiKey) {
      ticketsStatus.textContent = 'Desk365 hostname saved. Add your API key to finish connecting.';
    } else {
      ticketsStatus.textContent = '';
    }
  }
}

function initializeTickets() {
  if (initializeTicketsPromise) {
    return initializeTicketsPromise;
  }

  initializeTicketsPromise = performInitializeTickets().finally(() => {
    initializeTicketsPromise = null;
  });
  return initializeTicketsPromise;
}

saveApiKeyBtn.addEventListener('click', async () => {
  if (!(window.currentStorageStatus && window.currentStorageStatus.sharedDataAvailable)) {
    ticketsStatus.textContent = window.currentStorageStatus.message || 'Desk365 settings are unavailable while shared data is offline.';
    return;
  }

  const key = document.getElementById('api-key-input').value.trim();
  const domain = document.getElementById('domain-input').value.trim();
  if (!key || !domain) {
    ticketsStatus.textContent = 'Enter both the Desk365 hostname and API key.';
    return;
  }

  saveApiKeyBtn.disabled = true;
  ticketsStatus.textContent = 'Saving Desk365 credentials…';

  try {
    await window.callCommand('save_ticket_settings', { desk365Domain: domain });
    await window.callCommand('save_secure_api_key', { apiKey: key });
    document.getElementById('api-key-input').value = '';
    await initializeTickets();
  } catch (error) {
    ticketsStatus.textContent = error.message || 'Could not save Desk365 credentials.';
    setTicketSetupVisible(true);
  } finally {
    saveApiKeyBtn.disabled = !(window.currentStorageStatus && window.currentStorageStatus.sharedDataAvailable);
  }
});

changeApiKeyBtn.addEventListener('click', async () => {
  const confirmed = confirm('Clear the saved Desk365 API key from secure storage?');
  if (!confirmed) return;

  try {
    await window.callCommand('clear_secure_api_key');
    document.getElementById('api-key-input').value = '';
    await initializeTickets();
  } catch (error) {
    ticketsStatus.textContent = error.message || 'Could not clear the saved API key.';
  }
});

refreshBtn.addEventListener('click', () => fetchAndRenderTickets({ force: true, forceFull: true }));
showHiddenCheckbox.addEventListener('change', () => {
  try {
    renderTickets();
  } catch (error) {
    console.error('Failed to render tickets:', error);
    ticketsStatus.textContent = error.message || 'Could not render the tickets list.';
  }
});

window.addEventListener('storage-status-changed', async () => {
  await initializeTickets();
});

window.addEventListener('shared-data-changed', async (event) => {
  const files = event.detail && Array.isArray(event.detail.files) ? event.detail.files : [];
  if (files.includes('config.json')) {
    await initializeTickets();
    return;
  }

  if (files.includes('hidden-tickets.json')) {
    if (hiddenTicketSaveInFlight) {
      return;
    }
    try {
      const hiddenData = await window.callCommand('load_hidden_tickets');
      applyHiddenTicketDocument(hiddenData);
      renderTickets();
    } catch (error) {
      ticketsStatus.textContent = error.message || 'Could not refresh hidden ticket state.';
    }
  }
});

window.addEventListener('shared-data-reconcile', async () => {
  try {
    const hiddenData = await window.callCommand('load_hidden_tickets');
    applyHiddenTicketDocument(hiddenData);
    renderTickets();
  } catch (error) {
    console.error('Failed to reconcile hidden tickets:', error);
  }
});

initializeTickets();
