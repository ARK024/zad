'use strict';

// Tabs Navigation
const tabLinks = document.querySelectorAll('.tab-link');
const tabContents = document.querySelectorAll('.tab-content');
tabLinks.forEach(link => {
  link.addEventListener('click', () => {
    tabLinks.forEach(l => l.classList.remove('active'));
    tabContents.forEach(c => c.classList.remove('active'));
    link.classList.add('active');
    document.getElementById(link.dataset.tab).classList.add('active');
  });
});

const gid = id => document.getElementById(id);
const statusEl = gid('status');

function showStatus(msg, ok) {
  statusEl.textContent = msg;
  statusEl.style.color = ok === false ? 'var(--danger)' : 'var(--accent)';
  clearTimeout(showStatus._t);
  showStatus._t = setTimeout(() => { statusEl.textContent = ''; }, 2800);
}

async function load() {
  // Populate Fonts
  try {
    const systemFonts = await window.api.invoke('m:get-fonts');
    const selFont = gid('selFontFam');
    systemFonts.forEach(f => {
      if (!Array.from(selFont.options).some(o => o.value === f)) {
        const opt = document.createElement('option');
        opt.value = f;
        opt.textContent = f;
        selFont.appendChild(opt);
      }
    });
  } catch (e) {
    console.warn('Failed to load system fonts:', e);
  }

  // Load Hadith & System Config
  const s = await S.get();

  const mem = s.index || 0;
  const total = s.total || 1;
  const p = Math.min((mem / total) * 100, 100);
  gid('sMem').textContent = mem;
  gid('sLeft').textContent = total - mem;
  gid('sTotal').textContent = total;
  gid('barFill').style.width = p.toFixed(1) + '%';
  gid('barLbl').textContent = p.toFixed(1) + '%';

  const iv = String(s.interval || 30);
  const selIv = gid('selIv');
  const opt = Array.from(selIv.options).find(o => o.value === iv);
  if (opt) {
    selIv.value = iv;
    gid('customBox').style.display = 'none';
  } else {
    selIv.value = 'custom';
    gid('inpCustom').value = s.interval;
    gid('customBox').style.display = 'block';
  }

  gid('slFont').value = s.fontSize || 22;
  gid('lblFont').textContent = gid('slFont').value;
  gid('selFontFam').value = s.fontFamily || "'QuranFont', 'Traditional Arabic'";
  gid('cSanad').value = s.cSanad || '#5d7a69';
  gid('cMatn').value = s.cMatn || '#182820';
  gid('cTakhrij').value = s.cTakhrij || '#1a9850';
  gid('cSharh').value = s.cSharh || '#b35900';
  gid('chkDark').checked = s.theme === 'dark';
  document.body.classList.toggle('dark', s.theme === 'dark');
  gid('chkAuto').checked = !!s.autoLaunch;
  gid('selAppMode').value = s.appMode || 'sequential';
  gid('hReviewEnabled').checked = !!s.hReviewEnabled;
  gid('hReviewDays').value = s.hReviewDays || 7;

  // Load Quran Config
  const q = await window.api.invoke('q:store:get', {
    dailyGoal: 1,
    memorizationInterval: 10,
    widgetSize: 'medium',
    fontSizePx: 26,
    reviewEnabled: false,
    recentReviewEnabled: false,
    reviewDays: 7,
    reviewPagesPerSession: 10,
    hideHeader: false,
    currentQuranPage: 1,
  });
  gid('quranDailyGoal').value = q.dailyGoal;
  gid('quranInterval').value = q.memorizationInterval;
  gid('quranWidgetSize').value = q.widgetSize;
  gid('quranFontSize').value = q.fontSizePx;
  gid('quranLblFont').textContent = q.fontSizePx;
  gid('quranReviewEnabled').checked = !!q.reviewEnabled;
  gid('quranRecentReviewEnabled').checked = !!q.recentReviewEnabled;
  gid('quranReviewDays').value = q.reviewDays || 7;
  gid('quranReviewPagesPerSession').value = q.reviewPagesPerSession || 10;
  gid('quranHideHeader').checked = !!q.hideHeader;
  gid('quranStartPage').value = q.currentQuranPage || 1;

  // Load Quran Stats
  const qd = await window.api.invoke('q:store:get', null);
  try {
    const memCount = new Set([...(qd.memorizedPages || []), ...(qd.preloadedPages || [])]).size;
    gid('qMem').textContent = memCount.toString();
    gid('qTotal').textContent = (qd.totalReadCount || 0).toString();
    gid('qStreak').textContent = (qd.dailyStreak || 0).toString();
  } catch (e) { /* ignore */ }
}

gid('selIv').addEventListener('change', () => {
  gid('customBox').style.display = gid('selIv').value === 'custom' ? 'block' : 'none';
});
gid('slFont').addEventListener('input', () => {
  gid('lblFont').textContent = gid('slFont').value;
});
gid('quranFontSize').addEventListener('input', () => {
  gid('quranLblFont').textContent = gid('quranFontSize').value;
});

gid('chkDark').addEventListener('change', () => {
  document.body.classList.toggle('dark', gid('chkDark').checked);
});

// Save All
gid('btnSave').addEventListener('click', async () => {
  let interval;
  if (gid('selIv').value === 'custom') {
    interval = parseInt(gid('inpCustom').value, 10);
    if (!Number.isFinite(interval) || interval < 1) {
      showStatus('أدخل وقت حديث صحيح', false);
      return;
    }
  } else {
    interval = parseInt(gid('selIv').value, 10);
  }

  const qGoal = parseInt(gid('quranDailyGoal').value, 10) || 1;
  const qInterv = parseInt(gid('quranInterval').value, 10) || 10;

  const qData = {
    dailyGoal: qGoal,
    memorizationInterval: qInterv,
    widgetSize: gid('quranWidgetSize').value,
    fontSizePx: parseInt(gid('quranFontSize').value, 10) || 26,
    reviewEnabled: gid('quranReviewEnabled').checked,
    recentReviewEnabled: gid('quranRecentReviewEnabled').checked,
    reviewDays: parseInt(gid('quranReviewDays').value, 10) || 7,
    reviewPagesPerSession: parseInt(gid('quranReviewPagesPerSession').value, 10) || 10,
    hideHeader: gid('quranHideHeader').checked,
  };

  await window.api.invoke('q:store:set', qData);

  const hData = {
    interval,
    fontSize: parseInt(gid('slFont').value, 10) || 22,
    fontFamily: gid('selFontFam').value,
    cSanad: gid('cSanad').value,
    cMatn: gid('cMatn').value,
    cTakhrij: gid('cTakhrij').value,
    cSharh: gid('cSharh').value,
    theme: gid('chkDark').checked ? 'dark' : 'light',
    autoLaunch: gid('chkAuto').checked,
    appMode: gid('selAppMode').value,
    hReviewEnabled: gid('hReviewEnabled').checked,
    hReviewDays: parseInt(gid('hReviewDays').value, 10) || 7,
  };
  const res = await S.save(hData);

  showStatus(res.ok ? 'تم حفظ كافة الإعدادات بنجاح!' : (res.msg || 'خطأ'), res.ok);
  if (res.ok) {
    await load();
    window.api.invoke('m:recalculate-sequence');
  }
});

// Hadith actions
gid('btnShowNow').addEventListener('click', () => {
  S.showNow();
  showStatus('📖 تم عرض الحديث');
});

gid('btnReset').addEventListener('click', async () => {
  if (!confirm('هل تريد الرجوع إلى الحديث الأول؟')) return;
  const res = await S.reset();
  if (res.ok) {
    await load();
    showStatus('🔄 تمت إعادة البداية');
  }
});

gid('btnJump').addEventListener('click', async () => {
  const n = parseInt(gid('inpJump').value, 10);
  if (Number.isFinite(n) && n >= 1) {
    const res = await S.jump(n - 1);
    if (res.ok) {
      await load();
      showStatus(`انتقل للحديث ${n}`);
    }
  }
});

// Reset Quran Data
gid('btnResetQuranData').addEventListener('click', async () => {
  if (!confirm('تحذير: هل أنت متأكد من مسح كافة بيانات القرآن (التقدم، الحفظ، العداد)؟ لا يمكن التراجع عن هذا الإجراء.')) return;
  await window.api.invoke('q:store:clear');
  await load();
  showStatus('تم مسح بيانات القرآن', true);
});

// Reset geometries
gid('btnResetGeo').addEventListener('click', async () => {
  const res1 = await window.api.invoke('s:reset-quran-geometry');
  const res2 = await S.resetGeometry();
  showStatus(res1.ok && res2.ok ? 'تمت إعادة الموضع للافتراضي' : 'خطأ', res1.ok && res2.ok);
});

// Backup
gid('btnBackup').addEventListener('click', async () => {
  const res = await S.backup();
  showStatus(res.ok ? 'تم حفظ النسخة الاحتياطية' : 'تم الإلغاء أو فشل الحفظ', res.ok || null);
});

gid('btnRestore').addEventListener('click', async () => {
  if (!confirm('سيتم استبدال إعداداتك الحالية بالنسخة الاحتياطية. هل تريد المتابعة؟')) return;
  const res = await S.restore();
  if (res.ok) {
    await load();
    showStatus('تم الاستيراد بنجاح!');
  } else {
    showStatus(res.err || 'فشل الاستيراد', false);
  }
});

// Quran actions
gid('btnSetStartPage').addEventListener('click', async () => {
  const p = parseInt(gid('quranStartPage').value, 10);
  if (!p || p < 1 || p > 604) {
    showStatus('صفحة غير صحيحة', false);
    return;
  }
  await window.api.invoke('q:store:set', { currentQuranPage: p });
  showStatus('تم تعيين صفحة البداية لـ ' + p);
});

gid('btnPreload').addEventListener('click', async () => {
  const from = parseInt(gid('preloadFrom').value, 10);
  const to = parseInt(gid('preloadTo').value, 10);
  if (!from || !to || from < 1 || to < from || to > 604) {
    showStatus('نطاق غير صحيح', false);
    return;
  }

  const pages = [];
  for (let i = from; i <= to; i++) pages.push(i);

  const current = await window.api.invoke('q:store:get', { memorizedPages: [] });
  const combined = [...new Set([...(current.memorizedPages || []), ...pages])].sort((a, b) => a - b);
  await window.api.invoke('q:store:set', { memorizedPages: combined });
  showStatus('تم إضافة ' + pages.length + ' صفحة للمحفوظ');
});

gid('btnResetQuranGeo').addEventListener('click', async () => {
  const res = await window.api.invoke('s:reset-quran-geometry');
  showStatus(res.ok ? 'تمت إعادة الموضع للافتراضي' : 'خطأ', res.ok);
});

// Search
let searchTimer = null;
let searchGen = 0; // generation counter to prevent stale results
const inpSearch = gid('inpSearch');
const results = gid('results');
inpSearch.addEventListener('input', () => {
  clearTimeout(searchTimer);
  const q = inpSearch.value.trim();
  if (!q) {
    results.style.display = 'none';
    return;
  }
  const gen = ++searchGen;
  searchTimer = setTimeout(async () => {
    const list = await S.search(q);
    if (gen === searchGen) renderResults(list, q); // skip stale results
  }, 280);
});

function escapeHTML(s) {
  const d = document.createElement('div');
  d.textContent = s;
  return d.innerHTML;
}

function renderResults(list, q) {
  results.style.display = 'block';
  if (!list.length) {
    results.innerHTML = '<div class="no-res">لا توجد نتائج لـ «' + escapeHTML(q) + '»</div>';
    return;
  }
  results.innerHTML = list
    .map(
      r =>
        '<div class="r-item" data-i="' + r.index + '">' +
        '<div class="r-meta">' + (r.chapter ? escapeHTML(r.chapter) + ' · ' : '') + escapeHTML(r.narrator || '') + '  (#' + (r.index + 1) + ')</div>' +
        '<div class="r-text">' + escapeHTML(r.preview) + '…</div></div>',
    )
    .join('');
  results.querySelectorAll('.r-item').forEach(el => {
    el.addEventListener('click', async () => {
      const res = await S.jump(parseInt(el.dataset.i, 10));
      if (res.ok) {
        await load();
        S.showNow();
        results.style.display = 'none';
        inpSearch.value = '';
        showStatus('الحديث ' + (parseInt(el.dataset.i, 10) + 1));
      }
    });
  });
}

document.addEventListener('click', e => {
  if (!e.target.closest('.search-box') && !e.target.closest('.results')) {
    results.style.display = 'none';
  }
});

// ── Pause feature ──────────────────────────────────────────────────────────
function endOfTodayMs() {
  const d = new Date();
  d.setHours(23, 59, 59, 999);
  return d.getTime();
}

function fmtPause(ts) {
  if (!ts || ts <= Date.now()) return '';
  const ms = ts - Date.now();
  const mins = Math.round(ms / 60000);
  if (mins >= 60) {
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return 'متوقف لـ ' + h + ' س' + (m ? ' و ' + m + ' د' : '');
  }
  return 'متوقف لـ ' + mins + ' دقيقة';
}

async function refreshPauseUI() {
  const cfg = await window.api.invoke('q:store:get', { pausedUntil: 0 });
  const ts = (cfg && cfg.pausedUntil) || 0;
  const el = gid('pauseStatus');
  const btn = gid('btnResume');
  if (ts && ts > Date.now()) {
    el.textContent = fmtPause(ts);
    el.style.color = '#b85c00';
    btn.style.display = 'block';
  } else {
    el.textContent = 'التطبيق نشط الآن';
    el.style.color = '#2e7d32';
    btn.style.display = 'none';
  }
}

document.querySelectorAll('.pause-btn').forEach(btn => {
  btn.addEventListener('click', async () => {
    const mins = parseInt(btn.dataset.mins, 10);
    const ts = mins === -1 ? endOfTodayMs() : Date.now() + mins * 60000;
    await window.api.invoke('q:store:set', { pausedUntil: ts });
    await refreshPauseUI();
    showStatus('تم إيقاف التذكيرات مؤقتاً');
  });
});

gid('btnResume').addEventListener('click', async () => {
  await window.api.invoke('q:store:set', { pausedUntil: 0 });
  await refreshPauseUI();
  showStatus('تم استئناف التطبيق');
});

setInterval(refreshPauseUI, 30000);
refreshPauseUI();

load();
