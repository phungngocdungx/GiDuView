// GiDuView - Application Logic & Tauri 2.0 Client Interface

const tauriCore = window.__TAURI__?.core || window.__TAURI__?.tauri;
const tauriEvent = window.__TAURI__?.event;

// State
let isConnected = false;
let isHosting = false;
let isViewOnly = false;
let remoteWidth = 1920;
let remoteHeight = 1080;
let lastMoveTime = 0;

// DOM Elements
const navTabs = document.querySelectorAll('.nav-tab');
const tabPanes = document.querySelectorAll('.tab-pane');
const connectForm = document.getElementById('connect-form');
const partnerIdInput = document.getElementById('partner-id');
const partnerPwInput = document.getElementById('partner-password');
const togglePwBtn = document.getElementById('toggle-password-visibility');
const optViewOnly = document.getElementById('opt-view-only');
const optClipboard = document.getElementById('opt-clipboard');
const btnConnect = document.getElementById('btn-connect');
const recentList = document.getElementById('recent-list');

// Host elements
const hostIdEl = document.getElementById('host-id');
const hostPwEl = document.getElementById('host-password');
const btnToggleHost = document.getElementById('btn-toggle-host');
const hostBtnLabel = document.getElementById('host-btn-label');
const hostLiveIndicator = document.getElementById('host-live-indicator');
const btnCopyId = document.getElementById('btn-copy-id');
const btnCopyPw = document.getElementById('btn-copy-pw');

// Permissions elements
const badgeScreen = document.getElementById('badge-screen');
const badgeInput = document.getElementById('badge-input');
const btnReqScreen = document.getElementById('btn-req-screen');
const btnReqInput = document.getElementById('btn-req-input');
const btnRefreshPerms = document.getElementById('btn-refresh-perms');

// Session Viewport elements
const sessionOverlay = document.getElementById('session-overlay');
const sessionToolbar = document.getElementById('session-toolbar');
const sessionLoading = document.getElementById('session-loading');
const sessionLoadingText = document.getElementById('session-loading-text');
const partnerDisplay = document.getElementById('session-partner-display');
const resDisplay = document.getElementById('session-res-display');
const remoteCanvas = document.getElementById('remote-canvas');
const ctx = remoteCanvas.getContext('2d');

const tbBtnControl = document.getElementById('tb-btn-control');
const tbIconControl = document.getElementById('tb-icon-control');
const tbLabelControl = document.getElementById('tb-label-control');
const tbQualitySelect = document.getElementById('tb-quality-select');
const tbBtnCad = document.getElementById('tb-btn-cad');
const tbBtnFullscreen = document.getElementById('tb-btn-fullscreen');
const tbBtnDisconnect = document.getElementById('tb-btn-disconnect');

// Helpers for invoking Tauri backend
async function invoke(cmd, args = {}) {
  if (tauriCore?.invoke) {
    return await tauriCore.invoke(cmd, args);
  }
  console.warn(`[Mock Invoke] ${cmd}`, args);
  return null;
}

// Tab Switching
navTabs.forEach(tab => {
  tab.addEventListener('click', () => {
    const target = tab.dataset.tab;
    navTabs.forEach(t => t.classList.remove('active'));
    tabPanes.forEach(p => p.classList.remove('active'));
    tab.classList.add('active');
    document.getElementById(target)?.classList.add('active');

    if (target === 'tab-permissions') {
      checkPermissionsStatus();
    } else if (target === 'tab-host') {
      loadHostInfo();
    }
  });
});

// Password visibility toggle
togglePwBtn.addEventListener('click', () => {
  partnerPwInput.type = partnerPwInput.type === 'password' ? 'text' : 'password';
});

// Partner ID formatter
partnerIdInput.addEventListener('input', (e) => {
  let v = e.target.value.replace(/\D/g, '');
  if (v.length > 3 && v.length <= 6) {
    v = v.slice(0, 3) + ' ' + v.slice(3);
  } else if (v.length > 6) {
    v = v.slice(0, 3) + ' ' + v.slice(3, 6) + ' ' + v.slice(6, 9);
  }
  e.target.value = v;
});

// Copy Credentials
btnCopyId.addEventListener('click', () => {
  const text = hostIdEl.innerText.replace(/\s+/g, '');
  navigator.clipboard.writeText(text);
  btnCopyId.innerText = 'Copied!';
  setTimeout(() => (btnCopyId.innerText = 'Copy'), 1500);
});

btnCopyPw.addEventListener('click', () => {
  navigator.clipboard.writeText(hostPwEl.innerText);
  btnCopyPw.innerText = 'Copied!';
  setTimeout(() => (btnCopyPw.innerText = 'Copy'), 1500);
});

// Connect Form
connectForm.addEventListener('submit', async (e) => {
  e.preventDefault();
  const id = partnerIdInput.value.replace(/\s+/g, '');
  const password = partnerPwInput.value.trim();

  if (!id || !password) return;

  btnConnect.disabled = true;
  partnerDisplay.innerText = partnerIdInput.value;
  sessionOverlay.style.display = 'flex';
  sessionLoading.style.display = 'flex';
  sessionLoadingText.innerText = `Connecting to ${id}...`;

  try {
    isViewOnly = optViewOnly.checked;
    updateControlUI(isViewOnly);

    await invoke('connect_remote', {
      id,
      password,
      options: {
        viewOnly: isViewOnly,
        clipboard: optClipboard.checked,
        quality: tbQualitySelect.value,
      },
    });

    isConnected = true;
  } catch (err) {
    alert(`Connection failed: ${err}`);
    sessionOverlay.style.display = 'none';
  } finally {
    btnConnect.disabled = false;
  }
});

// Disconnect
tbBtnDisconnect.addEventListener('click', async () => {
  await invoke('disconnect_remote');
  sessionOverlay.style.display = 'none';
  isConnected = false;
});

// View / Control Toggle
tbBtnControl.addEventListener('click', async () => {
  isViewOnly = !isViewOnly;
  await invoke('set_view_only', { viewOnly: isViewOnly });
  updateControlUI(isViewOnly);
});

function updateControlUI(viewOnly) {
  if (viewOnly) {
    tbIconControl.innerText = '👁️';
    tbLabelControl.innerText = 'View Only';
    tbBtnControl.style.background = 'rgba(255, 149, 0, 0.3)';
  } else {
    tbIconControl.innerText = '🎮';
    tbLabelControl.innerText = 'Control';
    tbBtnControl.style.background = 'rgba(255, 255, 255, 0.1)';
  }
}

// Special Keys
tbBtnCad.addEventListener('click', async () => {
  await invoke('send_special_key', { action: 'ctrl_alt_del' });
});

// Fullscreen
tbBtnFullscreen.addEventListener('click', () => {
  if (!document.fullscreenElement) {
    sessionOverlay.requestFullscreen().catch(() => {});
  } else {
    document.exitFullscreen().catch(() => {});
  }
});

// Hosting Toggle
btnToggleHost.addEventListener('click', async () => {
  if (!isHosting) {
    try {
      const res = await invoke('start_host');
      if (res) {
        hostIdEl.innerText = res.id;
        hostPwEl.innerText = res.password;
        isHosting = true;
        updateHostUI(true);
      }
    } catch (e) {
      alert(`Could not start hosting: ${e}`);
    }
  } else {
    await invoke('stop_host');
    isHosting = false;
    updateHostUI(false);
  }
});

function updateHostUI(hosting) {
  if (hosting) {
    btnToggleHost.className = 'btn btn-danger';
    hostBtnLabel.innerText = 'Stop Screen Sharing';
    hostLiveIndicator.style.display = 'flex';
  } else {
    btnToggleHost.className = 'btn btn-success';
    hostBtnLabel.innerText = 'Start Screen Sharing';
    hostLiveIndicator.style.display = 'none';
  }
}

async function loadHostInfo() {
  try {
    const info = await invoke('get_host_info');
    if (info) {
      hostIdEl.innerText = info.id;
      hostPwEl.innerText = info.password;
      isHosting = info.is_hosting;
      updateHostUI(isHosting);
    }
  } catch (e) {
    console.error(e);
  }
}

// Permissions Check
async function checkPermissionsStatus() {
  try {
    const perms = await invoke('get_system_permissions');
    if (perms) {
      updatePermBadge(badgeScreen, perms.screen_recording);
      updatePermBadge(badgeInput, perms.accessibility);
    }
  } catch (e) {
    console.error(e);
  }
}

function updatePermBadge(el, granted) {
  if (granted) {
    el.innerText = 'Granted';
    el.className = 'badge granted';
  } else {
    el.innerText = 'Permission Needed';
    el.className = 'badge denied';
  }
}

btnReqScreen.addEventListener('click', async () => {
  await invoke('request_permissions', { permission: 'screen' });
  setTimeout(checkPermissionsStatus, 1000);
});

btnReqInput.addEventListener('click', async () => {
  await invoke('request_permissions', { permission: 'accessibility' });
  setTimeout(checkPermissionsStatus, 1000);
});

btnRefreshPerms.addEventListener('click', checkPermissionsStatus);

// Canvas Frame Rendering
function renderFrame(payload) {
  const { width, height, format, data } = payload;
  if (remoteCanvas.width !== width || remoteCanvas.height !== height) {
    remoteCanvas.width = width;
    remoteCanvas.height = height;
    remoteWidth = width;
    remoteHeight = height;
    resDisplay.innerText = `${width}x${height}`;
  }

  const img = new Image();
  img.onload = () => {
    ctx.drawImage(img, 0, 0);
    sessionLoading.style.display = 'none';
  };
  img.src = `data:image/${format};base64,${data}`;
}

// Mouse Event Forwarding
function getCanvasCoords(e) {
  const rect = remoteCanvas.getBoundingClientRect();
  const scaleX = remoteCanvas.width / rect.width;
  const scaleY = remoteCanvas.height / rect.height;
  const x = Math.round((e.clientX - rect.left) * scaleX);
  const y = Math.round((e.clientY - rect.top) * scaleY);
  return {
    x: Math.max(0, Math.min(remoteWidth - 1, x)),
    y: Math.max(0, Math.min(remoteHeight - 1, y)),
  };
}

remoteCanvas.addEventListener('mousemove', (e) => {
  if (isViewOnly) return;
  const now = performance.now();
  if (now - lastMoveTime < 16) return; // 60hz throttling
  lastMoveTime = now;

  const { x, y } = getCanvasCoords(e);
  let wparam = 0;
  if (e.buttons & 1) wparam |= 1;
  if (e.buttons & 2) wparam |= 2;
  if (e.shiftKey) wparam |= 4;
  if (e.ctrlKey) wparam |= 8;
  if (e.buttons & 4) wparam |= 16;

  invoke('send_mouse_move', { x, y, wparam });
});

remoteCanvas.addEventListener('mousedown', (e) => {
  if (isViewOnly) return;
  remoteCanvas.focus();
  const { x, y } = getCanvasCoords(e);
  const lparam = (y << 16) | (x & 0xffff);
  let msg = 0x201; // WM_LBUTTONDOWN
  if (e.button === 2) msg = 0x204; // WM_RBUTTONDOWN
  if (e.button === 1) msg = 0x207; // WM_MBUTTONDOWN

  invoke('send_mouse_button', { msg, wparam: 0, lparam });
});

remoteCanvas.addEventListener('mouseup', (e) => {
  if (isViewOnly) return;
  const { x, y } = getCanvasCoords(e);
  const lparam = (y << 16) | (x & 0xffff);
  let msg = 0x202; // WM_LBUTTONUP
  if (e.button === 2) msg = 0x205; // WM_RBUTTONUP
  if (e.button === 1) msg = 0x208; // WM_MBUTTONUP

  invoke('send_mouse_button', { msg, wparam: 0, lparam });
});

remoteCanvas.addEventListener('wheel', (e) => {
  if (isViewOnly) return;
  e.preventDefault();
  const { x, y } = getCanvasCoords(e);
  const delta = e.deltaY < 0 ? 120 : -120;
  invoke('send_mouse_wheel', { delta, keyFlags: 0, x, y });
}, { passive: false });

remoteCanvas.addEventListener('contextmenu', (e) => {
  e.preventDefault();
});

// Keyboard Event Forwarding
window.addEventListener('keydown', (e) => {
  if (!isConnected || isViewOnly) return;
  if (document.activeElement === partnerIdInput || document.activeElement === partnerPwInput) return;

  // Prevent browser default actions for common shortcuts
  if (['Tab', 'Backspace', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Space'].includes(e.code)) {
    e.preventDefault();
  }

  const text = e.key.length === 1 ? e.key : null;
  invoke('send_key_event', { code: e.code, pressed: true, text });
});

window.addEventListener('keyup', (e) => {
  if (!isConnected || isViewOnly) return;
  if (document.activeElement === partnerIdInput || document.activeElement === partnerPwInput) return;

  invoke('send_key_event', { code: e.code, pressed: false, text: null });
});

// Register Tauri event listeners
if (tauriEvent?.listen) {
  tauriEvent.listen('session-status', (event) => {
    sessionLoadingText.innerText = event.payload;
  });

  tauriEvent.listen('session-connected', (event) => {
    const { width, height, partner_version } = event.payload;
    remoteWidth = width;
    remoteHeight = height;
    resDisplay.innerText = `${width}x${height}`;
    sessionLoading.style.display = 'none';
  });

  tauriEvent.listen('session-frame', (event) => {
    renderFrame(event.payload);
  });

  tauriEvent.listen('session-closed', (event) => {
    alert(`Session ended: ${event.payload}`);
    sessionOverlay.style.display = 'none';
    isConnected = false;
  });

  tauriEvent.listen('host-status', (event) => {
    console.log(`Host status: ${event.payload}`);
  });
}

// Initial Load
window.addEventListener('DOMContentLoaded', () => {
  loadHostInfo();
  checkPermissionsStatus();

  // Load recents
  invoke('get_recent_partners').then(recents => {
    if (recents && recents.length > 0) {
      recentList.innerHTML = '';
      recents.forEach(id => {
        const item = document.createElement('div');
        item.className = 'recent-item';
        item.innerHTML = `<span>${id}</span><button class="btn btn-secondary btn-sm">Use</button>`;
        item.querySelector('button').addEventListener('click', () => {
          partnerIdInput.value = id;
          partnerPwInput.focus();
        });
        recentList.appendChild(item);
      });
    }
  });
});
