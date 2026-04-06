// AppBar disabled — the SHAppBarMessage approach doesn't work reliably
// from Electron because it requires a native Win32 message loop handler.
// The app still works fine as an always-on-top sidebar; users just need
// to manually avoid maximizing over it, or snap windows to the left.

module.exports = {
  reserveRight() {},
  releaseReserve() {},
  isRegistered() { return false; },
};
