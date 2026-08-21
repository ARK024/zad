'use strict';

let IDX = 0, TOTAL = 1, FZ = 22;

function toAr(n) {
  return String(n).replace(/\d/g, d => '٠١٢٣٤٥٦٧٨٩'[d]);
}

// Cache DOM elements once to avoid repeated lookups
const $ = id => document.getElementById(id);
const _el = {
  scrollCtn: $('scrollCtn'),
  badge: $('badge'),
  prog: $('prog'),
  counter: $('counter'),
  pct: $('pct'),
  btnForgot: $('btnForgot'),
  btnMem: $('btnMem'),
  meta: $('meta'),
  chTag: $('chTag'),
  narr: $('narr'),
  hdth: $('hdth'),
  btnPrev: $('btnPrev'),
  btnNext: $('btnNext'),
  fzD: $('fzD'),
  fzU: $('fzU'),
  btnClose: $('btnClose'),
  btnHide: $('btnHide'),
};

let _firstLoad = true;
let _fadeTimer = null; // prevent race conditions

W.onHadith(function (d) {
  function applyData() {
    IDX = d.index;
    TOTAL = d.total;
    FZ = d.fontSize || 22;
    document.body.classList.toggle('dark', d.theme === 'dark');

    _el.badge.textContent =
      (d.isReview ? '🔄 مراجعة — ' : '') + toAr(IDX + 1) + ' / ' + toAr(TOTAL);
    const p = ((IDX + 1) / TOTAL) * 100;
    _el.prog.style.width = p.toFixed(2) + '%';
    _el.counter.textContent = (IDX + 1) + ' / ' + TOTAL;
    _el.pct.textContent = p.toFixed(1) + '%';

    if (d.isReview) {
      _el.btnForgot.style.display = 'block';
      _el.btnMem.textContent = 'تذكرته ✅';
    } else {
      _el.btnForgot.style.display = 'none';
      _el.btnMem.textContent = 'حفظته ✅';
    }

    if (d.chapter || d.narrator) {
      _el.meta.style.display = 'block';
      _el.chTag.textContent = d.chapter || '';
      _el.chTag.style.display = d.chapter ? 'inline-block' : 'none';
      _el.narr.textContent = d.narrator ? 'الراوي: ' + d.narrator : '';
      _el.narr.style.display = d.narrator ? 'block' : 'none';
    } else {
      _el.meta.style.display = 'none';
    }

    const root = document.documentElement;
    if (d.fontFamily) {
      root.style.setProperty('--font-fam', d.fontFamily + ", 'Segoe UI', Tahoma, Arial, sans-serif");
    } else {
      root.style.setProperty('--font-fam', "'Segoe UI', Tahoma, Arial, sans-serif");
    }

    if (d.cSanad) root.style.setProperty('--sanad-c', d.cSanad);
    if (d.cMatn) root.style.setProperty('--matn-c', d.cMatn);
    if (d.cTakhrij) root.style.setProperty('--takhrij-c', d.cTakhrij);
    if (d.cSharh) root.style.setProperty('--sharh-c', d.cSharh);

    _el.hdth.style.fontSize = FZ + 'px';
    _el.hdth.style.fontFamily = 'var(--font-fam)';

    let fText = d.text || '';
    if (d.matn || d.sanad || d.takhrij || d.sharh) {
      const parts = [
        { text: d.matn, cls: 'matn-txt' },
        { text: d.sanad, cls: 'sanad-txt' },
        { text: d.takhrij, cls: 'takhrij-txt' },
        { text: d.sharh, cls: 'sharh-txt' },
      ].filter(p => p.text && p.text.trim().length > 0);

      parts.sort((a, b) => b.text.length - a.text.length);

      for (const part of parts) {
        const idx = fText.indexOf(part.text);
        if (idx !== -1) {
          fText =
            fText.substring(0, idx) +
            '<span class="' + part.cls + '" style="color:var(--' + part.cls.replace('-txt', '-c') + ')">' +
            part.text +
            '</span>' +
            fText.substring(idx + part.text.length);
        }
      }
      _el.hdth.innerHTML = fText.replace(/\n/g, '<br/>');
    } else {
      _el.hdth.textContent = d.text;
    }

    _el.scrollCtn.scrollTop = 0;
    _el.btnPrev.disabled = IDX <= 0;
    _el.btnNext.disabled = IDX >= TOTAL - 1;
  }

  if (_firstLoad) {
    _firstLoad = false;
    applyData();
  } else {
    // Cancel any pending fade to prevent race conditions
    if (_fadeTimer) clearTimeout(_fadeTimer);
    _el.scrollCtn.classList.add('fade-out');
    _fadeTimer = setTimeout(function () {
      applyData();
      _el.scrollCtn.classList.remove('fade-out');
      _el.scrollCtn.classList.add('fade-in');
      _fadeTimer = setTimeout(function () {
        _el.scrollCtn.classList.remove('fade-in');
        _fadeTimer = null;
      }, 160);
    }, 120);
  }
});

// Font size controls — persist change via IPC
_el.fzD.onclick = function () {
  FZ = Math.max(12, FZ - 1);
  _el.hdth.style.fontSize = FZ + 'px';
};
_el.fzU.onclick = function () {
  FZ = Math.min(72, FZ + 1);
  _el.hdth.style.fontSize = FZ + 'px';
};
_el.btnClose.onclick = function () { W.hide(); };
_el.btnHide.onclick = function () { W.hide(); };
_el.btnMem.onclick = function () { W.memorized(IDX); };
_el.btnForgot.onclick = function () { W.forgot(IDX); };
_el.btnNext.onclick = function () {
  if (!_el.btnNext.disabled) W.next(IDX);
};
_el.btnPrev.onclick = function () {
  if (!_el.btnPrev.disabled) W.prev(IDX);
};

document.addEventListener('keydown', function (e) {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
  if (e.key === 'Escape') W.hide();
  if (e.key === 'Enter') W.memorized(IDX);
  if (e.key === 'ArrowRight' && !_el.btnPrev.disabled) W.prev(IDX);
  if (e.key === 'ArrowLeft' && !_el.btnNext.disabled) W.next(IDX);
});
