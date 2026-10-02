/**
 * Session storage for the mini-program.
 *
 * The IAM SDK family for MP runtimes has not been generated yet, so the
 * session holds the platform-issued bearer token only (dev convenience entry
 * on the login page). wx.login / code2session replaces this once the identity
 * contract lands (see docs/decisions.md). Logout/account switch clears the
 * platform storage key entirely.
 */
const STORAGE_KEY = "sdkwork-order-mp-session-token";

function getToken() {
  try {
    return wx.getStorageSync(STORAGE_KEY) || "";
  } catch (error) {
    return "";
  }
}

function setToken(token) {
  try {
    wx.setStorageSync(STORAGE_KEY, token || "");
  } catch (error) {
    // storage unavailable: keep the session in memory only
  }
}

function clearToken() {
  try {
    wx.removeStorageSync(STORAGE_KEY);
  } catch (error) {
    // ignore
  }
}

function isLoggedIn() {
  return Boolean(getToken());
}

module.exports = { getToken, setToken, clearToken, isLoggedIn };
