

function abortAndHide() {
  const loader = document.getElementById('__loading');
  if (loader) loader.remove();
  if (window.api && window.api.invoke) {
    window.api.invoke('q:window:hide').catch(() => {});
  }
}

function toArabicNumerals(num) {
  if (num === undefined || num === null || isNaN(num)) return '';
  const arabicNumbers = ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'];
  return num.toString().split('').map(digit => arabicNumbers[digit] ?? digit).join('');
}


let _dismissingWidget = false;
// eslint-disable-next-line no-unused-vars

const WIDGET_WIDTHS = { small: '280px', medium: '380px', large: '480px', xlarge: '580px' };

function cleanupWidget(widget) {
  if (!widget) return;
  (widget.__cleanup || []).forEach(fn => fn());
  widget.__cleanup = [];
  widget.remove();
}

// لف كل كلمة في span.test-word عشان الـ blur يشتغل كلمة كلمة
function wrapWordsForTestMode(bodyEl) {
  if (!bodyEl) return;
  const walker = document.createTreeWalker(bodyEl, NodeFilter.SHOW_TEXT, null);
  const textNodes = [];
  while (walker.nextNode()) textNodes.push(walker.currentNode);

  for (const node of textNodes) {
    const words = node.textContent.split(/(\s+)/);
    if (words.length <= 1 && !words[0].trim()) continue;
    const frag = document.createDocumentFragment();
    for (const w of words) {
      if (!w.trim()) {
        frag.appendChild(document.createTextNode(w));
      } else {
        const span = document.createElement('span');
        span.className = 'test-word';
        span.textContent = w;
        frag.appendChild(span);
      }
    }
    node.parentNode.replaceChild(frag, node);
  }
}



// Media functions removed — no video/audio elements in desktop app

let _fontInjected = false;

async function injectCustomFont() {
  if (_fontInjected) return;
  const possibleFonts = [
    'quran-font.ttf', 'quran-font.woff2', 'quran-font.woff',
    'quran.ttf', 'quran.woff2', 'quran.woff'
  ];

  let foundFontUrl = null;
  let foundFontFormat = null;

  for (const fontName of possibleFonts) {
    const url = fontName;
    try {
      const response = await fetch(url, { method: 'HEAD' });
      if (response.ok) {
        foundFontUrl = url;
        if (fontName.endsWith('.ttf')) foundFontFormat = 'truetype';
        else if (fontName.endsWith('.woff2')) foundFontFormat = 'woff2';
        else if (fontName.endsWith('.woff')) foundFontFormat = 'woff';
        break;
      }
    } catch (e) {
      // خطأ في تحميل الخط — غير حرج
    }
  }

  if (foundFontUrl) {
    const style = document.createElement('style');
    document.head.appendChild(style);
    style.textContent = `
      @font-face {
        font-family: 'QuranFont';
        src: url('${foundFontUrl}') format('${foundFontFormat}');
        font-display: swap;
        unicode-range: U+0621-06FF, U+0750-077F, U+08A0-08FF, U+FB50-FDFF, U+FE70-FEFF;
      }
    `;
  }
  _fontInjected = true;
}

// Widget positioning handled natively by Tauri window manager

// البيانات تُجلَب من background.js عبر messaging
async function getPageAyahsFromBG(pageNumber) {
  return window.api.invoke('q:bg:message', { type: 'getPageAyahs', page: pageNumber });
}

async function getMultiplePagesFromBG(pageNumbers) {
  return window.api.invoke('q:bg:message', { type: 'getMultiplePages', pages: pageNumbers });
}

function bindHeaderToggle(toggleBtn, collapseBtn, headerEl) {
  toggleBtn?.addEventListener('click', async (e) => {
    e.stopPropagation();
    if (headerEl) headerEl.style.display = 'flex';
    if (toggleBtn) toggleBtn.style.display = 'none';
    try {
      await window.api.invoke('q:store:set', { hideHeader: false });
    } catch (e) {
      console.warn('Quran Widget: failed to save hideHeader state', e);
    }
  });

  collapseBtn?.addEventListener('click', async (e) => {
    e.stopPropagation();
    if (headerEl) headerEl.style.display = 'none';
    if (toggleBtn) toggleBtn.style.display = 'flex';
    try {
      await window.api.invoke('q:store:set', { hideHeader: true });
    } catch (e) {
      console.warn('Quran Widget: failed to save hideHeader state', e);
    }
  });
}

async function showRecentReviewPage(recentData, widgetSize, hideHeader, _attempts = 0) {
  if (_attempts >= recentData.pages.length) {
    // All attempts exhausted — fall through to normal memorization
    const data = await window.api.invoke('q:store:get', { currentQuranPage: 1, widgetSize: 'medium', hideHeader: false });
    await showNewMemorizationPage(data.currentQuranPage, data.widgetSize || widgetSize, data.hideHeader || hideHeader);
    return;
  }

  const pps = recentData.pagesPerSession > 0
    ? recentData.pagesPerSession
    : (recentData.pages.length - recentData.currentIndex);

  // اجمع صفحات الجلسة عبر الـ background
  const pageNums = [];
  for (let i = recentData.currentIndex; i < Math.min(recentData.currentIndex + pps, recentData.pages.length); i++) {
    pageNums.push(recentData.pages[i]);
  }
  const sessionPages = await getMultiplePagesFromBG(pageNums);

  if (!sessionPages || sessionPages.length === 0) {
    await StorageManager.incrementRecentReviewIndex();
    const newData = await StorageManager.getTodayRecentReviewData();
    if (newData.currentIndex < newData.pages.length) {
      await showRecentReviewPage(newData, widgetSize, hideHeader, _attempts + 1);
    }
    return;
  }

  // الصفحة التالية للمعاينة
  const lastSessionPageNum = sessionPages[sessionPages.length - 1].pageNum;
  const nextPageNum = lastSessionPageNum >= 604 ? 1 : lastSessionPageNum + 1;
  let nextPagePreview = null;
  const nextPd = await getPageAyahsFromBG(nextPageNum);
  if (nextPd) {
    nextPagePreview = { pageNum: nextPageNum, surahTitle: nextPd.surahTitle, firstAyahHtml: nextPd.firstAyahHtml || '' };
  }

  await injectRecentReviewWidget(sessionPages, recentData, nextPagePreview, widgetSize, hideHeader);
}

async function injectRecentReviewWidget(sessionPages, recentData, nextPagePreview, widgetSize = 'medium', hideHeader = false) {
  const existingWidget = document.getElementById('quran-memorization-widget');
  if (existingWidget) existingWidget.remove();

  const fontData = await window.api.invoke('q:store:get', { fontSizePx: 26, testModeEnabled: false, hideHeader: false });
  const testModeOn = fontData.testModeEnabled || false;
  const isHeaderHidden = fontData.hideHeader !== undefined ? !!fontData.hideHeader : !!hideHeader;

  const widget = document.createElement('div');
  widget.id = 'quran-memorization-widget';

  const firstPage = sessionPages[0];
  const surahTitle = firstPage.pageData.surahTitle;
  const pageNumber = firstPage.pageNum;
  const sessionCount = sessionPages.length;

  const progressPct = Math.min(Math.round(((recentData.currentIndex + sessionCount) / recentData.totalToday) * 100), 100);
  const reviewText = sessionCount > 1
    ? `قريب: ${recentData.currentIndex + 1}–${recentData.currentIndex + sessionCount} من ${recentData.totalToday}`
    : `مراجعة ${recentData.currentIndex + 1} من ${recentData.totalToday}`;

  const testBtnHtml = `<button class="quran-widget-test-btn${testModeOn ? ' active' : ''}" id="quran-test-toggle" title="وضع الاختبار">👁️</button>`;

  const fullHeader = `
    <div class="quran-widget-header quran-widget-recent-header" id="quran-recent-header-content" style="${isHeaderHidden ? 'display:none;' : 'display:flex;'}">
      <div class="quran-widget-header-top">
        <span class="quran-widget-surah-name">⚡ قريب - سورة ${surahTitle}</span>
        <div class="quran-widget-header-controls">
          <span class="quran-widget-page-number">صفحة ${toArabicNumerals(pageNumber)}</span>
          ${testBtnHtml}
          <button class="quran-widget-header-btn" id="quran-header-collapse-btn" title="تصغير الهيدر">▲</button>
        </div>
      </div>
      <div class="quran-widget-review-progress">
        <div class="quran-widget-review-stats">
          <span>${reviewText}</span>
          <span>آخر ٧ أيام</span>
        </div>
        <div class="quran-widget-progress-container">
          <div class="quran-widget-progress-bar-wrapper">
            <div class="quran-widget-progress-bar quran-widget-recent-bar" style="width: ${progressPct}%"></div>
          </div>
          <div class="quran-widget-progress-text">${recentData.currentIndex + 1} / ${recentData.totalToday}</div>
        </div>
      </div>
    </div>
    <div class="quran-widget-header-collapsed recent" id="quran-header-toggle" title="توسيع الهيدر" style="${isHeaderHidden ? 'display:flex;' : 'display:none;'}">
      <span>⚡ ${surahTitle} — صفحة <span class="collapsed-page-num">${toArabicNumerals(pageNumber)}</span></span>
      <button class="quran-widget-header-btn" title="توسيع الهيدر">▼</button>
    </div>
  `;

  const bodyHtml = sessionPages.map((sp, idx) => {
    const sep = idx > 0
      ? `<div class="quran-widget-page-divider">— صفحة ${toArabicNumerals(sp.pageNum)} —</div>`
      : '';
    return sep + sp.pageData.ayahTextHtml;
  }).join('');

  const nextHtml = nextPagePreview ? `
    <div class="quran-widget-next-ayah">
      <div class="quran-widget-next-ayah-label">التالية: سورة ${nextPagePreview.surahTitle} — صفحة ${toArabicNumerals(nextPagePreview.pageNum)}</div>
      <div class="quran-widget-next-ayah-text">${nextPagePreview.firstAyahHtml}</div>
    </div>
  ` : '';

  widget.innerHTML = `
    ${fullHeader}
    <div class="quran-widget-body">${bodyHtml}${nextHtml}</div>
    <div class="quran-widget-footer">
      <button class="quran-widget-btn quran-widget-btn-skip"        id="quran-btn-recent-skip">إعادة 🔁</button>
      <button class="quran-widget-btn quran-widget-btn-done-recent" id="quran-btn-recent-done">تم المراجعة ✅</button>
    </div>
    <div class="quran-widget-success" id="quran-success-msg"></div>
  `;

  const _recentBody = widget.querySelector('.quran-widget-body');
  if (_recentBody) _recentBody.style.fontSize = fontData.fontSizePx + 'px';

  // تطبيق وضع الاختبار لو مفعّل
  if (testModeOn) widget.classList.add('quran-widget-test-active');

  document.body.appendChild(widget);

  // لف الكلمات في span عشان الـ blur
  if (_recentBody) wrapWordsForTestMode(_recentBody);

  // زر وضع الاختبار
  document.getElementById('quran-test-toggle')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    widget.classList.toggle('quran-widget-test-active');
    const isActive = widget.classList.contains('quran-widget-test-active');
    e.currentTarget.classList.toggle('active', isActive);
    await window.api.invoke('q:store:set', { testModeEnabled: isActive });
  });

  const toggleBtn = widget.querySelector('#quran-header-toggle');
  const collapseBtn = widget.querySelector('#quran-header-collapse-btn');
  const headerEl = widget.querySelector('#quran-recent-header-content');
  bindHeaderToggle(toggleBtn, collapseBtn, headerEl);

  document.getElementById('quran-btn-recent-skip')?.addEventListener('click', async () => {
    try {
      await StorageManager.retryRecentReviewPage();
      _dismissingWidget = true;
      await window.api.invoke('q:store:set', { lastCompletedTime: Date.now() });
    } catch (e) {
      console.warn('Quran Widget: recent-skip error', e);
      _dismissingWidget = true;
    }
    widget.classList.add('hiding');
    setTimeout(() => { cleanupWidget(widget); _dismissingWidget = false; }, 300);
  });

  document.getElementById('quran-btn-recent-done')?.addEventListener('click', async () => {
    let newIndex = recentData.currentIndex;
    let allDone = false;
    try {
      const _rrd = await window.api.invoke('q:store:get', { totalReadCount: 0 });
      await window.api.invoke('q:store:set', { totalReadCount: _rrd.totalReadCount + sessionCount });

      for (let i = 0; i < sessionCount; i++) {
        newIndex = await StorageManager.incrementRecentReviewIndex();
      }
      allDone = newIndex >= recentData.pages.length;
    } catch (e) {
      console.warn('Quran Widget: recent-done error', e);
    }

    const successMsg = document.getElementById('quran-success-msg');
    if (successMsg) {
      successMsg.innerHTML = allDone
        ? `<div>🎉 أتممت المراجعة القريبة!</div><div style="font-size:18px;margin-top:10px;color:#e67e22;">أحسنت 🔥</div>`
        : `<div>أحسنت!</div><div style="font-size:18px;margin-top:10px;color:#e67e22;">استمر في المراجعة 🔥</div>`;
      successMsg.classList.add('show');
    }

    setTimeout(async () => {
      try {
        _dismissingWidget = true;
        await window.api.invoke('q:store:set', { lastCompletedTime: Date.now() });
      } catch (e) {
        console.warn('Quran Widget: recent-done finalize error', e);
        _dismissingWidget = true;
      }
      widget.classList.add('hiding');
      setTimeout(() => { cleanupWidget(widget); _dismissingWidget = false; }, 300);
    }, 1500);
  });
}


async function initQuranWidget() {
  
  if (localStorage.getItem('showQuranWidget') !== 'true') {
    return; // Do nothing on startup
  }
  localStorage.removeItem('showQuranWidget');

  try {

    await injectCustomFont();
    await StorageManager.loadDayStartHour();

    try {
      await StorageManager.initData();
    } catch (e) { return abortAndHide(); }

    let data;
    try {
      data = await window.api.invoke('q:store:get', {
        currentQuranPage: 1,
        memorizationInterval: 10,
        lastCompletedTime: 0,
        widgetSize: 'medium',
        hideHeader: false,
        reviewEnabled: false,
        recentReviewEnabled: false,
        pausedUntil: 0
      });
    } catch (e) { return abortAndHide(); }



    const widgetSize = data.widgetSize || 'medium';
    const hideHeader = data.hideHeader || false;

    

    if (data.reviewEnabled) {
      const reviewData = await StorageManager.getTodayReviewPages();

      if (reviewData.pages.length > 0 && reviewData.currentIndex < reviewData.pages.length) {
        await showReviewPage(reviewData, widgetSize, hideHeader);
        return;
      }
    }

    // المراجعة القريبة — تعمل بشكل مستقل (إعداد منفصل)
    if (data.recentReviewEnabled) {
      const recentData = await StorageManager.getTodayRecentReviewData();
      if (recentData.enabled && recentData.currentIndex < recentData.pages.length) {
        await showRecentReviewPage(recentData, widgetSize, hideHeader);
        return;
      }
    }

    await showNewMemorizationPage(data.currentQuranPage, widgetSize, hideHeader);

  } catch (error) { abortAndHide(); }
}

async function showReviewPage(reviewData, widgetSize, hideHeader, _attempts = 0) {
  if (_attempts >= reviewData.pages.length) {
    console.warn('Quran Widget: تعذّر تحميل أي صفحة مراجعة، الانتقال للحفظ');
    const data = await window.api.invoke('q:store:get', ['currentQuranPage', 'widgetSize', 'hideHeader']);
    await showNewMemorizationPage(data.currentQuranPage, data.widgetSize || 'medium', data.hideHeader || false);
    return;
  }

  const pps = reviewData.pagesPerSession > 0 ? reviewData.pagesPerSession : (reviewData.pages.length - reviewData.currentIndex);

  // اجمع الصفحات عبر الـ background
  const pageNums = [];
  for (let i = reviewData.currentIndex; i < Math.min(reviewData.currentIndex + pps, reviewData.pages.length); i++) {
    pageNums.push(reviewData.pages[i]);
  }
  const sessionPages = await getMultiplePagesFromBG(pageNums);

  if (!sessionPages || sessionPages.length === 0) {
    await StorageManager.incrementReviewIndex();
    const nd = await StorageManager.getTodayReviewPages();
    if (nd.currentIndex < nd.pages.length) {
      await showReviewPage(nd, widgetSize, hideHeader, _attempts + 1);
    } else {
      const data = await window.api.invoke('q:store:get', ['currentQuranPage', 'widgetSize', 'hideHeader']);
      await showNewMemorizationPage(data.currentQuranPage, data.widgetSize || 'medium', data.hideHeader || false);
    }
    return;
  }

  // الصفحة التالية للمعاينة
  const lastSessionPageNum = sessionPages[sessionPages.length - 1].pageNum;
  const nextPageNum = lastSessionPageNum >= 604 ? 1 : lastSessionPageNum + 1;
  let nextPagePreview = null;
  const nextPd = await getPageAyahsFromBG(nextPageNum);
  if (nextPd) {
    nextPagePreview = { pageNum: nextPageNum, surahTitle: nextPd.surahTitle, firstAyahHtml: nextPd.firstAyahHtml || '' };
  }

  await injectReviewWidget(sessionPages, reviewData, nextPagePreview, widgetSize, hideHeader);
}

async function showNewMemorizationPage(currentPage, widgetSize, hideHeader) {
  currentPage = parseInt(currentPage, 10) || 1;
  if (currentPage > 604) {
    currentPage = 1;
    await window.api.invoke('q:store:set', { currentQuranPage: 1 });
  }

  // نتخطى الصفحات المحفوظة مسبقاً (preloaded)
  const allMemorized = await StorageManager.getAllMemorizedPages();
  if (allMemorized.includes(currentPage)) {
    const memorizedSet = new Set(allMemorized);
    let searchPage = currentPage;
    let loopCount = 0;
    while (memorizedSet.has(searchPage) && loopCount < 604) {
      searchPage = searchPage >= 604 ? 1 : searchPage + 1;
      loopCount++;
    }
    if (loopCount < 604) {
      currentPage = searchPage;
      await window.api.invoke('q:store:set', { currentQuranPage: currentPage });
    }
  }

  const pageData = await getPageAyahsFromBG(currentPage);
  if (!pageData) {
    console.error('No Ayahs found for page', currentPage);
    abortAndHide();
    return;
  }

  const progress = await StorageManager.getDailyProgress();
  const pageStats = await StorageManager.getPageStats(currentPage);

  const nextPage = currentPage >= 604 ? 1 : currentPage + 1;
  const nextPd = await getPageAyahsFromBG(nextPage);
  let nextAyahPreview = null;
  if (nextPd) {
    nextAyahPreview = {
      text: '', number: 0, surah: nextPd.surahTitle, page: nextPage,
      _fullHtml: nextPd.firstAyahHtml
    };
  }

  await injectWidget(pageData.surahTitle, currentPage, pageData.ayahTextHtml, progress, pageStats, nextAyahPreview, widgetSize, hideHeader, pageData.ayahs);
}

async function injectReviewWidget(sessionPages, reviewData, nextPagePreview, widgetSize = 'medium', hideHeader = false) {
  const existingWidget = document.getElementById('quran-memorization-widget');
  if (existingWidget) existingWidget.remove();

  const _reviewFontData = await window.api.invoke('q:store:get', { fontSizePx: 26, testModeEnabled: false, hideHeader: false });
  const testModeOn = _reviewFontData.testModeEnabled || false;
  const isHeaderHidden = _reviewFontData.hideHeader !== undefined ? !!_reviewFontData.hideHeader : !!hideHeader;

  const widget = document.createElement('div');
  widget.id = 'quran-memorization-widget';

  // عنوان الهدر: أول سورة في الجلسة
  const firstPage = sessionPages[0];
  const surahTitle = firstPage.pageData.surahTitle;
  const pageNumber = firstPage.pageNum;
  const sessionCount = sessionPages.length;

  const globalIdx = reviewData.globalIndex || reviewData.currentIndex;
  const reviewProgressText = `بعيد: ${globalIdx + 1}–${globalIdx + sessionCount} من ${reviewData.totalToday}`;
  const dayProgressText = `يوم ${reviewData.dayIndex} من ${reviewData.totalDays}`;

  const testBtnHtml = `<button class="quran-widget-test-btn${testModeOn ? ' active' : ''}" id="quran-test-toggle" title="وضع الاختبار">👁️</button>`;

  const fullHeader = `
    <div class="quran-widget-header quran-widget-distant-header" id="quran-review-header-content" style="${isHeaderHidden ? 'display:none;' : 'display:flex;'}">
      <div class="quran-widget-header-top">
        <span class="quran-widget-surah-name">📅 بعيد - سورة ${surahTitle}</span>
        <div class="quran-widget-header-controls">
          <span class="quran-widget-page-number">صفحة ${toArabicNumerals(pageNumber)}</span>
          ${testBtnHtml}
          <button class="quran-widget-header-btn" id="quran-review-collapse-btn" title="تصغير الهيدر">▲</button>
        </div>
      </div>
      <div class="quran-widget-review-progress">
        <div class="quran-widget-review-stats">
          <span>${reviewProgressText}</span>
          <span>${dayProgressText}</span>
        </div>
        <div class="quran-widget-progress-container">
          <div class="quran-widget-progress-bar-wrapper">
            <div class="quran-widget-progress-bar quran-widget-distant-bar" style="width: ${Math.min(((globalIdx + sessionCount) / reviewData.totalToday) * 100, 100)}%"></div>
          </div>
          <div class="quran-widget-progress-text">${globalIdx + 1} / ${reviewData.totalToday}</div>
        </div>
      </div>
    </div>
    <div class="quran-widget-header-collapsed distant" id="quran-review-toggle" title="توسيع الهيدر" style="${isHeaderHidden ? 'display:flex;' : 'display:none;'}">
      <span>🔄 ${surahTitle} — صفحة <span class="collapsed-page-num">${toArabicNumerals(pageNumber)}</span></span>
      <button class="quran-widget-header-btn" title="توسيع الهيدر">▼</button>
    </div>
  `;

  // بناء محتوى كل الصفحات في الجلسة
  const bodyHtml = sessionPages.map((sp, idx) => {
    const sep = idx > 0
      ? `<div class="quran-widget-page-divider">— صفحة ${toArabicNumerals(sp.pageNum)} —</div>`
      : '';
    return sep + sp.pageData.ayahTextHtml;
  }).join('');

  const nextHtml = nextPagePreview ? `
    <div class="quran-widget-next-ayah">
      <div class="quran-widget-next-ayah-label">التالية: سورة ${nextPagePreview.surahTitle} — صفحة ${toArabicNumerals(nextPagePreview.pageNum)}</div>
      <div class="quran-widget-next-ayah-text">${nextPagePreview.firstAyahHtml}</div>
    </div>
  ` : '';

  widget.innerHTML = `
    ${fullHeader}
    <div class="quran-widget-body">
      ${bodyHtml}${nextHtml}
    </div>
    <div class="quran-widget-footer">
      <button class="quran-widget-btn quran-widget-btn-skip" id="quran-btn-skip">إعادة 🔁</button>
      <button class="quran-widget-btn quran-widget-btn-done-distant" id="quran-btn-done-distant">تم المراجعة ✅</button>
    </div>
    <div class="quran-widget-success" id="quran-success-msg"></div>
  `;

  // طبّق الخط قبل الإضافة للـ DOM
  const _reviewBody = widget.querySelector('.quran-widget-body');
  if (_reviewBody) _reviewBody.style.fontSize = _reviewFontData.fontSizePx + 'px';

  // تطبيق وضع الاختبار لو مفعّل
  if (testModeOn) widget.classList.add('quran-widget-test-active');

  document.body.appendChild(widget);

  // لف الكلمات في span عشان الـ blur
  if (_reviewBody) wrapWordsForTestMode(_reviewBody);

  // زر وضع الاختبار
  document.getElementById('quran-test-toggle')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    widget.classList.toggle('quran-widget-test-active');
    const isActive = widget.classList.contains('quran-widget-test-active');
    e.currentTarget.classList.toggle('active', isActive);
    await window.api.invoke('q:store:set', { testModeEnabled: isActive });
  });

  const revToggleBtn = widget.querySelector('#quran-review-toggle');
  const revCollapseBtn = widget.querySelector('#quran-review-collapse-btn');
  const revHeaderEl = widget.querySelector('#quran-review-header-content');
  bindHeaderToggle(revToggleBtn, revCollapseBtn, revHeaderEl);

  document.getElementById('quran-btn-skip')?.addEventListener('click', async () => {
    try {
      await StorageManager.retryReviewPage();
      _dismissingWidget = true;
      await window.api.invoke('q:store:set', { lastCompletedTime: Date.now() });
    } catch (e) {
      console.warn('Quran Widget: distant-skip error', e);
      _dismissingWidget = true;
    }
    widget.classList.add('hiding');
    setTimeout(() => {
      cleanupWidget(widget);
      _dismissingWidget = false;
    }, 300);
  });

  document.getElementById('quran-btn-done-distant')?.addEventListener('click', async () => {
    let newIndex = reviewData.currentIndex;
    let allDone = false;
    try {
      // زوّد totalReadCount بعدد الصفحات في الجلسة
      const _drd = await window.api.invoke('q:store:get', { totalReadCount: 0 });
      await window.api.invoke('q:store:set', { totalReadCount: _drd.totalReadCount + sessionCount });

      // نزوّد الـ index بعدد الصفحات اللي اتراجعت في هذه الجلسة
      for (let i = 0; i < sessionCount; i++) {
        newIndex = await StorageManager.incrementReviewIndex();
      }
      // تحديث sessionStart للجلسة التالية
      await StorageManager.advanceReviewSession(newIndex);
      const newReviewData = await StorageManager.getTodayReviewPages();
      allDone = newIndex >= newReviewData.totalToday;
    } catch (e) {
      console.warn('Quran Widget: distant-done error', e);
    }

    const successMsg = document.getElementById('quran-success-msg');
    if (successMsg) {
      successMsg.innerHTML = allDone
        ? `<div>🎉 أتممت مراجعة اليوم!</div><div style="font-size:18px;margin-top:10px;color:#27ae60;">أحسنت، واصل الاستمرار</div>`
        : `<div>ما شاء الله!</div><div style="font-size:18px;margin-top:10px;color:#27ae60;">تم الانتقال للصفحة التالية</div>`;
      successMsg.classList.add('show');
    }

    setTimeout(async () => {
      try {
        _dismissingWidget = true;
        await window.api.invoke('q:store:set', { lastCompletedTime: Date.now() });
      } catch (e) {
        console.warn('Quran Widget: distant-done finalize error', e);
        _dismissingWidget = true;
      }
      widget.classList.add('hiding');
      setTimeout(() => {
        cleanupWidget(widget);
        _dismissingWidget = false;
      }, 300);
    }, 1000);
  });
}

function isAyahNumberOrSymbol(word) {
  return /^[\s\uFD3E\uFD3F\u06DD\u06DE\u06DF\u06E0-\u06ED0-9\u0660-\u0669\(\)﴿﴾]+$/u.test(word);
}

function splitArabicWordHeadTail(word) {
  if (isAyahNumberOrSymbol(word)) return { head: word, tail: '' };
  const diacriticsRegex = /[\u064B-\u065F\u0670\u06D6-\u06ED]/;
  let head = '';
  let i = 0;
  while (i < word.length && !word[i].match(/\p{L}/u)) {
    head += word[i];
    i++;
  }
  if (i < word.length) {
    head += word[i];
    i++;
  }
  while (i < word.length && diacriticsRegex.test(word[i])) {
    head += word[i];
    i++;
  }
  return { head: head || word, tail: word.slice(i) };
}

function createChunks(ayahs, fullText) {
  const chunks = [{ id: 0, title: 'كامل الصفحة', text: fullText }];
  if (!ayahs || ayahs.length === 0) return chunks;

  const totalLen = ayahs.reduce((sum, a) => sum + a.length, 0);

  if (ayahs.length === 1) {
    const words = ayahs[0].split(/\s+/);
    if (words.length > 35) {
      const parts = ayahs[0].split(/([ۚۖۗۘۙۜ])/);
      if (parts.length >= 3) {
        const targetLen = totalLen / 2;
        let midPoint = 1;
        let runningLen = 0;
        for (let i = 0; i < parts.length; i += 2) {
          runningLen += parts[i].length;
          if (runningLen >= targetLen && i > 0) {
            midPoint = i + 1;
            break;
          }
        }
        if (midPoint >= parts.length) midPoint = parts.length - 2;
        chunks.push({ id: 1, title: 'مقطع ١', text: parts.slice(0, midPoint).join('').trim() });
        chunks.push({ id: 2, title: 'مقطع ٢', text: parts.slice(midPoint).join('').trim() });
      } else {
        const mid = Math.ceil(words.length / 2);
        chunks.push({ id: 1, title: 'مقطع ١', text: words.slice(0, mid).join(' ') });
        chunks.push({ id: 2, title: 'مقطع ٢', text: words.slice(mid).join(' ') });
      }
    }
    return chunks;
  }

  let numChunks = 2;
  if (totalLen > 300 && ayahs.length >= 3) numChunks = 3;
  if (ayahs.length >= 7) numChunks = 3;

  if (numChunks === 3 && ayahs.length >= 3) {
    let bestVariance = Infinity;
    let bestI = 1, bestJ = 2;
    for (let i = 1; i < ayahs.length; i++) {
      for (let j = i + 1; j < ayahs.length; j++) {
        const len1 = ayahs.slice(0, i).reduce((s, a) => s + a.length, 0);
        const len2 = ayahs.slice(i, j).reduce((s, a) => s + a.length, 0);
        const len3 = ayahs.slice(j).reduce((s, a) => s + a.length, 0);
        
        const mean = (len1 + len2 + len3) / 3;
        const variance = Math.pow(len1 - mean, 2) + Math.pow(len2 - mean, 2) + Math.pow(len3 - mean, 2);
        
        if (variance < bestVariance) {
          bestVariance = variance;
          bestI = i;
          bestJ = j;
        }
      }
    }
    chunks.push({ id: 1, title: 'مقطع ١', text: ayahs.slice(0, bestI).join(' ') });
    chunks.push({ id: 2, title: 'مقطع ٢', text: ayahs.slice(bestI, bestJ).join(' ') });
    chunks.push({ id: 3, title: 'مقطع ٣', text: ayahs.slice(bestJ).join(' ') });
  } else {
    let bestDiff = Infinity;
    let bestI = 1;
    for (let i = 1; i < ayahs.length; i++) {
        const len1 = ayahs.slice(0, i).reduce((s, a) => s + a.length, 0);
        const len2 = ayahs.slice(i).reduce((s, a) => s + a.length, 0);
        const diff = Math.abs(len1 - len2);
        if (diff < bestDiff) {
            bestDiff = diff;
            bestI = i;
        }
    }
    chunks.push({ id: 1, title: 'مقطع ١', text: ayahs.slice(0, bestI).join(' ') });
    chunks.push({ id: 2, title: 'مقطع ٢', text: ayahs.slice(bestI).join(' ') });
  }

  return chunks;
}

function renderAyahTextWithMasks(containerEl, text, level) {
  if (!containerEl) return;
  containerEl.innerHTML = '';
  const words = text.split(/(\s+)/);
  let wordIdx = 0;
  const frag = document.createDocumentFragment();

  for (const w of words) {
    if (!w.trim()) {
      frag.appendChild(document.createTextNode(w));
      continue;
    }
    const isSymbol = isAyahNumberOrSymbol(w);
    const span = document.createElement('span');
    span.className = 'test-word';
    span.dataset.idx = wordIdx;

    const { head, tail } = splitArabicWordHeadTail(w);
    const headSpan = document.createElement('span');
    headSpan.className = 'head-text';
    headSpan.textContent = head;

    const tailSpan = document.createElement('span');
    tailSpan.className = 'tail-text';
    tailSpan.textContent = tail;

    span.appendChild(headSpan);
    if (tail) span.appendChild(tailSpan);

    if (!isSymbol) {
      if (level === 1 && wordIdx % 4 === 2) {
        span.classList.add('masked-word');
      } else if (level === 2 && wordIdx % 2 === 1) {
        span.classList.add('masked-word');
      } else if (level === 3) {
        span.classList.add('keys-mode');
      } else if (level === 4) {
        span.classList.add('masked-word');
      }
      span.addEventListener('click', (e) => {
        e.stopPropagation();
        span.classList.toggle('revealed');
      });
      wordIdx++;
    }

    frag.appendChild(span);
  }

  containerEl.appendChild(frag);
}

async function injectWidget(surahTitle, pageNumber, ayahTextHtml, progress, pageStats, nextAyahPreview, widgetSize = 'medium', hideHeader = false, ayahs = []) {
  const existingWidget = document.getElementById('quran-memorization-widget');
  if (existingWidget) existingWidget.remove();

  const config = await window.api.invoke('q:store:get', {
    fontSizePx: 26,
    progressiveModeEnabled: false,
    progressiveLevel: 0,
    progressiveChunk: 0,
    hideHeader: false
  });

  const isHeaderHidden = config.hideHeader !== undefined ? !!config.hideHeader : !!hideHeader;
  let progressiveModeOn = !!config.progressiveModeEnabled;
  let currentLevel = Math.max(0, Math.min(4, config.progressiveLevel || 0));
  let currentChunkIdx = config.progressiveChunk || 0;

  const chunks = createChunks(ayahs, ayahTextHtml);
  if (currentChunkIdx >= chunks.length) currentChunkIdx = 0;

  const widget = document.createElement('div');
  widget.id = 'quran-memorization-widget';

  const safePageStats = pageStats || { today: 0, isMemorized: false };
  const readCountText = safePageStats.today > 0
    ? `قرأت ${safePageStats.today} ${safePageStats.today === 1 ? 'مرة' : 'مرات'} اليوم`
    : safePageStats.isMemorized ? 'محفوظة مسبقاً'
      : 'جديدة';

  const fullHeaderHtml = `
    <div class="quran-widget-header" id="quran-header-content" style="${isHeaderHidden ? 'display:none;' : 'display:flex;'}">
      <div class="quran-widget-header-top">
        <span class="quran-widget-surah-name">📖 سورة ${surahTitle}</span>
        <div class="quran-widget-header-controls">
          <button class="quran-widget-mode-btn${progressiveModeOn ? ' active' : ''}" id="quran-mode-toggle" title="التبديل بين وضع الحفظ العادي ووضع التدرج الذكي">
            ${progressiveModeOn ? '🧩 متدرج' : '📖 عادي'}
          </button>
          <span class="quran-widget-page-number">صفحة ${toArabicNumerals(pageNumber)}</span>
          <span class="quran-widget-read-badge">${readCountText}</span>
          <button class="quran-widget-header-btn" id="quran-header-collapse-btn" title="تصغير الهيدر">▲</button>
        </div>
      </div>
      <div class="quran-widget-progress-container">
        <div class="quran-widget-progress-bar-wrapper">
          <div class="quran-widget-progress-bar" style="width: ${progress.percentage}%"></div>
        </div>
        <div class="quran-widget-progress-row">
          <span>الهدف اليومي</span>
          <span>${progress.completed} / ${progress.goal}</span>
        </div>
      </div>
    </div>
    <div class="quran-widget-header-collapsed" id="quran-header-toggle" title="توسيع الهيدر" style="${isHeaderHidden ? 'display:flex;' : 'display:none;'}">
      <span>📖 سورة ${surahTitle} — صفحة <span class="collapsed-page-num">${toArabicNumerals(pageNumber)}</span></span>
      <button class="quran-widget-header-btn" title="توسيع الهيدر">▼</button>
    </div>
    <div class="quran-progressive-panel" id="quran-prog-panel" style="${progressiveModeOn ? '' : 'display:none;'}">
      <div class="quran-prog-row">
        <span style="font-weight:bold;color:var(--accent);">التلاشي:</span>
        <div class="quran-prog-pills" id="quran-prog-levels">
          <button class="quran-prog-pill${currentLevel === 0 ? ' active' : ''}" data-lvl="0">0% عادي</button>
          <button class="quran-prog-pill${currentLevel === 1 ? ' active' : ''}" data-lvl="1">25% خفيف</button>
          <button class="quran-prog-pill${currentLevel === 2 ? ' active' : ''}" data-lvl="2">50% متوسط</button>
          <button class="quran-prog-pill${currentLevel === 3 ? ' active' : ''}" data-lvl="3">مفاتيح</button>
          <button class="quran-prog-pill${currentLevel === 4 ? ' active' : ''}" data-lvl="4">100% غيباً</button>
        </div>
        <button class="quran-prog-btn-step" id="quran-prog-next-lvl" title="المستوى التالي">التالي ➡️</button>
      </div>
      ${chunks.length > 1 ? `
      <div class="quran-prog-row" style="margin-top:2px;">
        <span style="color:var(--muted);font-size:11px;">المقاطع:</span>
        <div class="quran-prog-pills" id="quran-prog-chunks">
          ${chunks.map((c, i) => `
            <button class="quran-prog-pill${currentChunkIdx === i ? ' active' : ''}" data-chunk="${i}">${c.title}</button>
          `).join('')}
        </div>
      </div>` : ''}
    </div>
  `;

  let nextAyahHtml = '';
  if (nextAyahPreview) {
    const previewContent = nextAyahPreview._fullHtml
      ? nextAyahPreview._fullHtml
      : `${nextAyahPreview.text} <span class="quran-widget-ayah-number">﴿${toArabicNumerals(nextAyahPreview.number)}﴾</span>`;
    nextAyahHtml = `
      <div class="quran-widget-next-ayah">
        <div class="quran-widget-next-ayah-text">
          ${previewContent}
        </div>
      </div>
    `;
  }

  widget.innerHTML = `
    ${fullHeaderHtml}
    <div class="quran-widget-body">
      <div class="quran-ayah-content" id="quran-ayah-container"></div>
      ${nextAyahHtml}
    </div>
    <div class="quran-widget-footer">
      <button class="quran-widget-btn quran-widget-btn-hide" id="quran-btn-hide">قرأتها</button>
      <button class="quran-widget-btn quran-widget-btn-done" id="quran-btn-done">أتممت حفظ الصفحة ✅</button>
    </div>
    <div class="quran-widget-success" id="quran-success-msg">
      <div>ما شاء الله!</div>
      <div style="font-size: 18px; margin-top: 10px; color: #27ae60;">تم الانتقال للصفحة التالية</div>
    </div>
  `;

  // طبّق الخط قبل الإضافة للـ DOM
  const fsPx = config.fontSizePx + 'px';
  const bodyEl = widget.querySelector('.quran-widget-body');
  if (bodyEl) bodyEl.style.fontSize = fsPx;
  const nextEl = widget.querySelector('.quran-widget-next-ayah-text');
  if (nextEl) nextEl.style.fontSize = fsPx;

  const ayahContainerEl = widget.querySelector('#quran-ayah-container');

  function updateProgressiveUI() {
    widget.querySelectorAll('#quran-prog-levels .quran-prog-pill').forEach(p => {
      p.classList.toggle('active', parseInt(p.dataset.lvl, 10) === currentLevel);
    });
    widget.querySelectorAll('#quran-prog-chunks .quran-prog-pill').forEach(p => {
      p.classList.toggle('active', parseInt(p.dataset.chunk, 10) === currentChunkIdx);
    });
    const activeText = progressiveModeOn ? chunks[currentChunkIdx].text : ayahTextHtml;
    const activeLvl = progressiveModeOn ? currentLevel : 0;
    renderAyahTextWithMasks(ayahContainerEl, activeText, activeLvl);
  }

  updateProgressiveUI();

  document.body.appendChild(widget);

  const toggleBtn = widget.querySelector('#quran-header-toggle');
  const collapseBtn = widget.querySelector('#quran-header-collapse-btn');
  const headerEl = widget.querySelector('#quran-header-content');
  bindHeaderToggle(toggleBtn, collapseBtn, headerEl);

  // تبديل الوضع بين عادي ومتدرج
  document.getElementById('quran-mode-toggle')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    progressiveModeOn = !progressiveModeOn;
    const btn = document.getElementById('quran-mode-toggle');
    const panel = document.getElementById('quran-prog-panel');
    if (btn) {
      btn.classList.toggle('active', progressiveModeOn);
      btn.textContent = progressiveModeOn ? '🧩 متدرج' : '📖 عادي';
    }
    if (panel) {
      panel.style.display = progressiveModeOn ? '' : 'none';
    }
    updateProgressiveUI();
    await window.api.invoke('q:store:set', { progressiveModeEnabled: progressiveModeOn });
  });

  // اختيار مستوى التلاشي
  widget.querySelectorAll('#quran-prog-levels .quran-prog-pill').forEach(btn => {
    btn.addEventListener('click', async () => {
      currentLevel = parseInt(btn.dataset.lvl, 10) || 0;
      updateProgressiveUI();
      await window.api.invoke('q:store:set', { progressiveLevel: currentLevel });
    });
  });

  // زر المستوى التالي
  document.getElementById('quran-prog-next-lvl')?.addEventListener('click', async () => {
    if (currentLevel < 4) {
      currentLevel++;
    } else if (chunks.length > 1 && currentChunkIdx < chunks.length - 1) {
      currentChunkIdx++;
      currentLevel = 0;
    } else {
      currentLevel = 0;
    }
    updateProgressiveUI();
    await window.api.invoke('q:store:set', {
      progressiveLevel: currentLevel,
      progressiveChunk: currentChunkIdx
    });
  });

  // اختيار المقطع
  widget.querySelectorAll('#quran-prog-chunks .quran-prog-pill').forEach(btn => {
    btn.addEventListener('click', async () => {
      currentChunkIdx = parseInt(btn.dataset.chunk, 10) || 0;
      updateProgressiveUI();
      await window.api.invoke('q:store:set', { progressiveChunk: currentChunkIdx });
    });
  });

  document.getElementById('quran-btn-hide')?.addEventListener('click', async () => {
    try {
      // إخفاء مؤقت — يُسجَّل كقراءة بدون حفظ
      const _hideData = await window.api.invoke('q:store:get', { totalReadCount: 0 });
      await window.api.invoke('q:store:set', { totalReadCount: _hideData.totalReadCount + 1 });
      _dismissingWidget = true;
      await window.api.invoke('q:store:set', { lastCompletedTime: Date.now() });
    } catch (e) {
      console.warn('Quran Widget: hide-btn storage error', e);
      _dismissingWidget = true;
    }
    widget.classList.add('hiding');
    setTimeout(() => {
      cleanupWidget(widget);
      _dismissingWidget = false;
    }, 300);
  });

  document.getElementById('quran-btn-done')?.addEventListener('click', async () => {
    const successMsg = document.getElementById('quran-success-msg');
    let undoClicked = false;
    let dismissTimeout = null;

    successMsg.innerHTML = `
      <div>ما شاء الله! 🎉</div>
      <div style="font-size: 15px; margin-top: 5px; color: #27ae60;">تم حفظ الصفحة والانتقال للتالية</div>
      <button id="quran-btn-undo-action" class="quran-widget-btn" style="margin-top:10px;padding:4px 14px;background:rgba(255,255,255,0.95);border:1.5px solid #c0392b;color:#c0392b;border-radius:18px;font-size:12px;font-weight:bold;cursor:pointer;display:inline-flex;align-items:center;gap:4px;box-shadow:0 2px 8px rgba(0,0,0,0.15);">
        ↩ تراجع (حفظت بالخطأ)
      </button>
    `;
    successMsg.classList.add('show');

    try {
      let nextPage = pageNumber + 1;
      if (nextPage > 604) nextPage = 1;

      await StorageManager.saveCompletedPage(pageNumber);

      _dismissingWidget = true;
      await window.api.invoke('q:store:set', {
        currentQuranPage: nextPage,
        lastCompletedTime: Date.now(),
        progressiveLevel: 0,
        progressiveChunk: 0
      });

      document.getElementById('quran-btn-undo-action')?.addEventListener('click', async (e) => {
        e.stopPropagation();
        undoClicked = true;
        if (dismissTimeout) clearTimeout(dismissTimeout);
        try {
          await StorageManager.unmemorizePage(pageNumber);
          await window.api.invoke('q:store:set', { currentQuranPage: pageNumber });
          successMsg.innerHTML = `<div style="color:#c0392b;font-weight:bold">تم التراجع بنجاح! ↩</div>`;
          setTimeout(() => {
            successMsg.classList.remove('show');
            _dismissingWidget = false;
          }, 800);
        } catch (err) {
          console.warn('Undo error', err);
        }
      });
    } catch (e) {
      console.warn('Quran Widget: done-btn storage error', e);
      _dismissingWidget = true;
    }

    dismissTimeout = setTimeout(() => {
      if (undoClicked) return;
      widget.classList.add('hiding');
      setTimeout(() => {
        cleanupWidget(widget);
        _dismissingWidget = false;
      }, 300);
    }, 3500);
  });
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initQuranWidget);
} else {
  initQuranWidget();
}

window.api.receive('q:store:changed', (changes) => {
  try {
    if (changes.lastCompletedTime) {
      if (_dismissingWidget) return;

      const widget = document.getElementById('quran-memorization-widget');
      if (widget) {
        widget.classList.add('hiding');
        setTimeout(() => cleanupWidget(widget), 300);
      }
    }

    if (changes.hideHeader !== undefined) {
      const val = (changes.hideHeader && typeof changes.hideHeader === 'object' && changes.hideHeader.newValue !== undefined) ? changes.hideHeader.newValue : changes.hideHeader;
      const isHidden = !!val;
      const widget = document.getElementById('quran-memorization-widget');
      if (widget) {
        const headerEl = widget.querySelector('#quran-header-content, #quran-recent-header-content, #quran-review-header-content');
        const toggleBtn = widget.querySelector('#quran-header-toggle, #quran-review-toggle');
        if (headerEl) headerEl.style.display = isHidden ? 'none' : 'flex';
        if (toggleBtn) toggleBtn.style.display = isHidden ? 'flex' : 'none';
      }
    }
  } catch (e) {
    console.warn('Quran Widget: storage.onChanged error', e);
  }
});
