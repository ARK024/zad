'use strict';

let IDX = 0, TOTAL = 1, FZ = 22;

function toAr(n) {
  return String(n).replace(/\d/g, d => '٠١٢٣٤٥٦٧٨٩'[d]);
}

let _firstLoad = true;

W.onHadith(function (d) {
  const scrollCtn = document.getElementById('scrollCtn');

  function applyData() {
    IDX = d.index;
    TOTAL = d.total;
    FZ = d.fontSize || 22;
    document.body.classList.toggle('dark', d.theme === 'dark');

    document.getElementById('badge').textContent =
      (d.isReview ? '🔄 مراجعة — ' : '') + toAr(IDX + 1) + ' / ' + toAr(TOTAL);
    const p = ((IDX + 1) / TOTAL) * 100;
    document.getElementById('prog').style.width = p.toFixed(2) + '%';
    document.getElementById('counter').textContent = (IDX + 1) + ' / ' + TOTAL;
    document.getElementById('pct').textContent = p.toFixed(1) + '%';

    const btnForgot = document.getElementById('btnForgot');
    const btnMem = document.getElementById('btnMem');
    if (d.isReview) {
      btnForgot.style.display = 'block';
      btnMem.textContent = 'تذكرته ✅';
    } else {
      btnForgot.style.display = 'none';
      btnMem.textContent = 'حفظته ✅';
    }

    const meta = document.getElementById('meta');
    if (d.chapter || d.narrator) {
      meta.style.display = 'block';
      const ct = document.getElementById('chTag');
      ct.textContent = d.chapter || '';
      ct.style.display = d.chapter ? 'inline-block' : 'none';
      const nr = document.getElementById('narr');
      nr.textContent = d.narrator ? 'الراوي: ' + d.narrator : '';
      nr.style.display = d.narrator ? 'block' : 'none';
    } else {
      meta.style.display = 'none';
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

    const hdthEl = document.getElementById('hdth');
    hdthEl.style.fontSize = FZ + 'px';
    hdthEl.style.fontFamily = 'var(--font-fam)';

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
      hdthEl.innerHTML = fText.replace(/\n/g, '<br/>');
    } else {
      hdthEl.textContent = d.text;
    }

    scrollCtn.scrollTop = 0;
    document.getElementById('btnPrev').disabled = IDX <= 0;
    document.getElementById('btnNext').disabled = IDX >= TOTAL - 1;
  }

  if (_firstLoad) {
    _firstLoad = false;
    applyData();
  } else {
    scrollCtn.classList.add('fade-out');
    setTimeout(function () {
      applyData();
      scrollCtn.classList.remove('fade-out');
      scrollCtn.classList.add('fade-in');
      setTimeout(function () {
        scrollCtn.classList.remove('fade-in');
      }, 160);
    }, 120);
  }
});

document.getElementById('fzD').onclick = function () {
  FZ = Math.max(12, FZ - 1);
  document.getElementById('hdth').style.fontSize = FZ + 'px';
};
document.getElementById('fzU').onclick = function () {
  FZ = Math.min(72, FZ + 1);
  document.getElementById('hdth').style.fontSize = FZ + 'px';
};
document.getElementById('btnClose').onclick = function () { W.hide(); };
document.getElementById('btnHide').onclick = function () { W.hide(); };
document.getElementById('btnMem').onclick = function () { W.memorized(IDX); };
document.getElementById('btnForgot').onclick = function () { W.forgot(IDX); };
document.getElementById('btnNext').onclick = function () {
  if (!document.getElementById('btnNext').disabled) W.next(IDX);
};
document.getElementById('btnPrev').onclick = function () {
  if (!document.getElementById('btnPrev').disabled) W.prev(IDX);
};

document.addEventListener('keydown', function (e) {
  if (e.key === 'Escape') W.hide();
  if (e.key === 'Enter') W.memorized(IDX);
  if (e.key === 'ArrowRight' && !document.getElementById('btnPrev').disabled) W.prev(IDX);
  if (e.key === 'ArrowLeft' && !document.getElementById('btnNext').disabled) W.next(IDX);
});
