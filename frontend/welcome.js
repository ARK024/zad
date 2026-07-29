'use strict';

document.getElementById('btnStart').addEventListener('click', function () {
  if (typeof Welcome === 'undefined' || !Welcome.done) {
    alert('IPC bridge not loaded — window.__TAURI__ = ' + typeof window.__TAURI__);
    return;
  }
  try {
    var p = Welcome.done({ autoLaunch: document.getElementById('chkAuto').checked });
    if (p && p.catch) {
      p.catch(function (e) {
        alert('welcome_done failed: ' + e);
      });
    }
  } catch (e) {
    alert('welcome_done threw: ' + e);
  }
});
