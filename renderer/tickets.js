// === Desk365 Tickets ===

let DESK365_BASE = '';
const POLL_INTERVAL = 5 * 60 * 1000; // 5 minutes

let config = null;
let hiddenTickets = [];
let currentTickets = [];
let seenTicketNumbers = new Set();
let pollTimer = null;

// DOM elements
const ticketsList = document.getElementById('tickets-list');
const ticketsStatus = document.getElementById('tickets-status');
const apiKeySetup = document.getElementById('api-key-setup');
const refreshBtn = document.getElementById('btn-refresh-tickets');
const showHiddenCheckbox = document.getElementById('show-hidden-tickets');

// Initialize tickets
async function initTickets() {
  config = await invoke('load_file', { filename: 'config.json' });
  const hiddenData = await invoke('load_file', { filename: 'hidden-tickets.json' });
  hiddenTickets = (hiddenData && hiddenData.tickets) || [];

  if (config && config.desk365Domain) {
    DESK365_BASE = `https://${config.desk365Domain}/app/tickets/ticketdetails?tktNum=`;
    document.getElementById('domain-input').value = config.desk365Domain;
  }

  if (config && config.apiKey) {
    apiKeySetup.style.display = 'none';
    fetchAndRenderTickets();
    startPolling();
  } else {
    apiKeySetup.style.display = 'block';
    ticketsList.innerHTML = '';
    ticketsStatus.textContent = '';
  }
}

// Save API key + domain
document.getElementById('btn-save-api-key').addEventListener('click', async () => {
  const key    = document.getElementById('api-key-input').value.trim();
  const domain = document.getElementById('domain-input').value.trim().replace(/^https?:\/\//, '');
  if (!key || !domain) return;

  config = { apiKey: key, desk365Domain: domain };
  DESK365_BASE = `https://${domain}/app/tickets/ticketdetails?tktNum=`;
  await invoke('save_file', { filename: 'config.json', data: config });
  apiKeySetup.style.display = 'none';
  fetchAndRenderTickets();
  startPolling();
});

// Enter key for API key input
document.getElementById('api-key-input').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') document.getElementById('btn-save-api-key').click();
});

// Fetch tickets from Desk365 API (HTTP is handled in Rust to avoid CORS)
async function fetchAndRenderTickets() {
  if (!config || !config.apiKey) return;

  ticketsStatus.textContent = 'Loading tickets...';
  refreshBtn.disabled = true;

  const result = await invoke('fetch_tickets', {
    apiKey: config.apiKey,
    desk365Domain: config.desk365Domain,
  });

  refreshBtn.disabled = false;

  if (!result.success) {
    ticketsStatus.textContent = 'Error: ' + result.error;
    apiKeySetup.style.display = 'block';
    document.getElementById('api-key-input').value = '';
    return;
  }

  const newTickets = result.tickets;

  // Notify for genuinely new tickets (not seen in previous poll)
  if (seenTicketNumbers.size > 0) {
    const brandNew = newTickets.filter(t => !seenTicketNumbers.has(t.TicketNumber));
    if (brandNew.length > 0) {
      const summary = brandNew.length === 1
        ? `#${brandNew[0].TicketNumber}: ${brandNew[0].Subject}`
        : `${brandNew.length} new tickets`;
      invoke('show_notification', { title: 'New Desk365 Tickets', body: summary });
    }
  }

  // Sort newest first
  newTickets.sort((a, b) => {
    const numA = parseInt(a.TicketNumber, 10) || 0;
    const numB = parseInt(b.TicketNumber, 10) || 0;
    return numB - numA;
  });

  seenTicketNumbers = new Set(newTickets.map(t => t.TicketNumber));
  currentTickets = newTickets;
  updateTabCount('tickets', currentTickets.length);

  ticketsStatus.textContent = `${newTickets.length} unresolved tickets (updated ${new Date().toLocaleTimeString()})`;
  renderTickets();
}

// Render ticket list
function renderTickets() {
  const showHidden = showHiddenCheckbox.checked;
  ticketsList.innerHTML = '';

  const filtered = showHidden
    ? currentTickets
    : currentTickets.filter(t => !hiddenTickets.includes(t.TicketNumber));

  if (filtered.length === 0) {
    ticketsList.innerHTML = '<div style="padding: 20px; text-align: center; color: var(--text-muted);">No tickets to show</div>';
    return;
  }

  filtered.forEach(ticket => {
    const isHidden = hiddenTickets.includes(ticket.TicketNumber);
    const card = document.createElement('div');
    card.className = 'ticket-card' + (isHidden ? ' hidden-ticket' : '');

    const info = document.createElement('div');
    info.className = 'ticket-info';
    info.addEventListener('click', () => {
      invoke('open_external', { url: DESK365_BASE + ticket.TicketNumber });
    });

    const num = document.createElement('span');
    num.className = 'ticket-number';
    num.textContent = '#' + ticket.TicketNumber;

    const subject = document.createElement('div');
    subject.className = 'ticket-subject';
    subject.textContent = ticket.Subject || '(no subject)';

    const meta = document.createElement('div');
    meta.className = 'ticket-meta';

    const status = document.createElement('span');
    status.className = 'ticket-status ' + (ticket.Status || '').toLowerCase().replace(/\s+/g, '');
    status.textContent = ticket.Status || 'Unknown';

    const priority = document.createElement('span');
    priority.textContent = ticket.Priority || '';

    const agent = document.createElement('span');
    agent.textContent = ticket.Agent ? ticket.Agent.split('@')[0] : '';

    meta.appendChild(status);
    if (ticket.Priority) meta.appendChild(priority);
    if (ticket.Agent) meta.appendChild(agent);

    info.appendChild(num);
    info.appendChild(subject);
    info.appendChild(meta);

    const hideBtn = document.createElement('button');
    hideBtn.className = 'ticket-hide-btn';
    hideBtn.textContent = isHidden ? '\u{1F441}' : '\u{1F6AB}';
    hideBtn.title = isHidden ? 'Unhide' : 'Hide';
    hideBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      toggleHideTicket(ticket.TicketNumber);
    });

    card.appendChild(info);
    card.appendChild(hideBtn);
    ticketsList.appendChild(card);
  });
}

// Toggle hide/unhide a ticket
async function toggleHideTicket(ticketNumber) {
  const idx = hiddenTickets.indexOf(ticketNumber);
  if (idx >= 0) {
    hiddenTickets.splice(idx, 1);
  } else {
    hiddenTickets.push(ticketNumber);
  }
  await invoke('save_file', {
    filename: 'hidden-tickets.json',
    data: { tickets: hiddenTickets },
  });
  renderTickets();
}

// Show hidden toggle
showHiddenCheckbox.addEventListener('change', renderTickets);

// Refresh button
refreshBtn.addEventListener('click', fetchAndRenderTickets);

// Change API key button
document.getElementById('btn-change-api-key').addEventListener('click', () => {
  apiKeySetup.style.display = 'block';
  document.getElementById('api-key-input').value = '';
  document.getElementById('api-key-input').focus();
});

// Polling
function startPolling() {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = setInterval(fetchAndRenderTickets, POLL_INTERVAL);
}

// Initialize on load
initTickets();
