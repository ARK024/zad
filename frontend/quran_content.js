

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

let pausedMediaElements = [];
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



function pauseAllMedia() {
  pausedMediaElements = [];

  const videos = document.querySelectorAll('video');
  const audios = document.querySelectorAll('audio');

  videos.forEach(video => {
    if (!video.paused) {
      pausedMediaElements.push(video);
      video.pause();
    }
  });

  audios.forEach(audio => {
    if (!audio.paused) {
      pausedMediaElements.push(audio);
      audio.pause();
    }
  });
}

function resumePausedMedia() {
  pausedMediaElements.forEach(media => {
    try {
      media.play();
    } catch (e) {
    }
  });
  pausedMediaElements = [];
}

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

function makeWidgetDraggable(_widget) {
  // Disabled in desktop app: Electron handles window dragging natively via CSS app-region
  return;
}

function makeWidgetResizable(_widget) {
  // Disabled in desktop app
  return;
}

async function loadWidgetPosition() {
  return null;
}

async function getStoredWidgetSize() {
  const size = await StorageManager.getWidgetSize();
  const vpW = window.innerWidth - 16;
  const vpH = window.innerHeight - 24;
  return {
    width: size.width ? `${Math.min(size.width, vpW)}px` : null,
    maxHeight: size.height ? `${Math.min(size.height, vpH)}px` : `${Math.min(vpH, 860)}px`
  };
}

function applyStoredSize(widget, sizeData) {
  if (sizeData.width) widget.style.width = sizeData.width;
  widget.style.maxHeight = sizeData.maxHeight;
}

// البيانات تُجلَب من background.js عبر messaging
async function getPageAyahsFromBG(pageNumber) {
  return window.api.invoke('q:bg:message', { type: 'getPageAyahs', page: pageNumber });
}

async function getMultiplePagesFromBG(pageNumbers) {
  return window.api.invoke('q:bg:message', { type: 'getMultiplePages', pages: pageNumbers });
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
  const nextIdx = recentData.currentIndex + sessionPages.length;
  let nextPagePreview = null;
  if (nextIdx < recentData.pages.length) {
    const nextPageNum = recentData.pages[nextIdx];
    const nextPd = await getPageAyahsFromBG(nextPageNum);
    if (nextPd) {
      nextPagePreview = { pageNum: nextPageNum, surahTitle: nextPd.surahTitle, firstAyahHtml: nextPd.firstAyahHtml || '' };
    }
  }

  await injectRecentReviewWidget(sessionPages, recentData, nextPagePreview, widgetSize, hideHeader);
}

async function injectRecentReviewWidget(sessionPages, recentData, nextPagePreview, widgetSize = 'medium', hideHeader = false) {
  const existingWidget = document.getElementById('quran-memorization-widget');
  if (existingWidget) existingWidget.remove();

  pauseAllMedia();

  const [position, sizeData, fontData] = await Promise.all([
    loadWidgetPosition(),
    getStoredWidgetSize(),
    window.api.invoke('q:store:get', { fontSizePx: 26, testModeEnabled: false })
  ]);
  const testModeOn = fontData.testModeEnabled || false;

  const widget = document.createElement('div');
  widget.id = 'quran-memorization-widget';

  applyStoredSize(widget, sizeData);
  if (!sizeData.width) widget.style.width = WIDGET_WIDTHS[widgetSize] || '380px';
  if (position) {
    widget.style.left = `${position.left}px`;
    widget.style.top = `${position.top}px`;
    widget.style.bottom = 'auto';
    widget.style.right = 'auto';
  }

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
    <div class="quran-widget-header quran-widget-recent-header" id="quran-recent-header-content">
      <div class="quran-widget-header-top">
        <span>⚡ قريب - سورة ${surahTitle}</span>
        <span style="display:flex;align-items:center;gap:6px;">صفحة ${toArabicNumerals(pageNumber)} ${testBtnHtml}</span>
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
  `;

  const headerContent = hideHeader ? `
    <div class="quran-widget-header-collapsed" id="quran-header-toggle" title="إظهار الهدر">
      <span>⚡ ${surahTitle} - صفحة <span class="collapsed-page-num">${toArabicNumerals(pageNumber)}</span></span>
    </div>
    ${fullHeader.replace('id="quran-recent-header-content"', 'id="quran-recent-header-content" style="display:none;"')}
  ` : fullHeader;

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
    ${headerContent}
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

  makeWidgetDraggable(widget);
  makeWidgetResizable(widget);

  // زر وضع الاختبار
  document.getElementById('quran-test-toggle')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    widget.classList.toggle('quran-widget-test-active');
    const isActive = widget.classList.contains('quran-widget-test-active');
    e.currentTarget.classList.toggle('active', isActive);
    await window.api.invoke('q:store:set', { testModeEnabled: isActive });
  });

  if (hideHeader) {
    const toggleBtn = document.getElementById('quran-header-toggle');
    const headerEl = document.getElementById('quran-recent-header-content');
    if (toggleBtn && headerEl) {
      toggleBtn.addEventListener('click', () => {
        if (headerEl.style.display === 'none') {
          headerEl.style.display = 'block';
          toggleBtn.style.display = 'none';
        }
      });
    }
  }

  document.getElementById('quran-btn-recent-skip')?.addEventListener('click', async () => {
    try {
      await StorageManager.retryRecentReviewPage();
      _dismissingWidget = true;
      await window.api.invoke('q:store:set', { lastCompletedTime: Date.now() });
    } catch (e) {
      console.warn('Quran Widget: recent-skip error', e);
      _dismissingWidget = true;
    }
    resumePausedMedia();
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
        resumePausedMedia();
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
  const nextIdx = reviewData.currentIndex + sessionPages.length;
  let nextPagePreview = null;
  if (nextIdx < reviewData.pages.length) {
    const nextPageNum = reviewData.pages[nextIdx];
    const nextPd = await getPageAyahsFromBG(nextPageNum);
    if (nextPd) {
      nextPagePreview = { pageNum: nextPageNum, surahTitle: nextPd.surahTitle, firstAyahHtml: nextPd.firstAyahHtml || '' };
    }
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

  await injectWidget(pageData.surahTitle, currentPage, pageData.ayahTextHtml, progress, pageStats, nextAyahPreview, widgetSize, hideHeader);
}

async function injectReviewWidget(sessionPages, reviewData, nextPagePreview, widgetSize = 'medium', hideHeader = false) {
  const existingWidget = document.getElementById('quran-memorization-widget');
  if (existingWidget) existingWidget.remove();

  pauseAllMedia();

  // ─── اجمع كل الـ async data قبل لمس الـ DOM ───
  const [position, sizeData, _reviewFontData] = await Promise.all([
    loadWidgetPosition(),
    getStoredWidgetSize(),
    window.api.invoke('q:store:get', { fontSizePx: 26, testModeEnabled: false })
  ]);
  const testModeOn = _reviewFontData.testModeEnabled || false;

  const widget = document.createElement('div');
  widget.id = 'quran-memorization-widget';

  applyStoredSize(widget, sizeData);
  if (!sizeData.width) widget.style.width = WIDGET_WIDTHS[widgetSize] || '380px';
  if (position) {
    widget.style.left = `${position.left}px`;
    widget.style.top = `${position.top}px`;
    widget.style.bottom = 'auto';
    widget.style.right = 'auto';
  }

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
    <div class="quran-widget-header quran-widget-distant-header" id="quran-review-header-content">
      <div class="quran-widget-header-top">
        <span>📅 بعيد - سورة ${surahTitle}</span>
        <span style="display:flex;align-items:center;gap:6px;">صفحة ${toArabicNumerals(pageNumber)} ${testBtnHtml}</span>
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
  `;

  const headerContent = hideHeader ? `
    <div class="quran-widget-header-collapsed" id="quran-header-toggle" title="إظهار الهدر">
      <span>🔄 ${surahTitle} - صفحة <span class="collapsed-page-num">${toArabicNumerals(pageNumber)}</span></span>
    </div>
    ${fullHeader.replace('id="quran-review-header-content"', 'id="quran-review-header-content" style="display:none;"')}
  ` : fullHeader;

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
    ${headerContent}
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

  makeWidgetDraggable(widget);
  makeWidgetResizable(widget);

  // زر وضع الاختبار
  document.getElementById('quran-test-toggle')?.addEventListener('click', async (e) => {
    e.stopPropagation();
    widget.classList.toggle('quran-widget-test-active');
    const isActive = widget.classList.contains('quran-widget-test-active');
    e.currentTarget.classList.toggle('active', isActive);
    await window.api.invoke('q:store:set', { testModeEnabled: isActive });
  });

  if (hideHeader) {
    const toggleBtn = document.getElementById('quran-header-toggle');
    const headerEl = document.getElementById('quran-review-header-content');
    if (toggleBtn && headerEl) {
      toggleBtn.addEventListener('click', () => {
        if (headerEl.style.display === 'none') {
          headerEl.style.display = 'block';
          toggleBtn.style.display = 'none';
        }
      });
    }
  }

  document.getElementById('quran-btn-skip')?.addEventListener('click', async () => {
    try {
      await StorageManager.retryReviewPage();
      _dismissingWidget = true;
      await window.api.invoke('q:store:set', { lastCompletedTime: Date.now() });
    } catch (e) {
      console.warn('Quran Widget: distant-skip error', e);
      _dismissingWidget = true;
    }
    resumePausedMedia();
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
        resumePausedMedia();
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

async function injectWidget(surahTitle, pageNumber, ayahTextHtml, progress, pageStats, nextAyahPreview, widgetSize = 'medium', hideHeader = false) {
  const existingWidget = document.getElementById('quran-memorization-widget');
  if (existingWidget) existingWidget.remove();

  pauseAllMedia();

  // ─── اجمع كل الـ async data قبل لمس الـ DOM ───
  const [position, sizeData, fontData] = await Promise.all([
    loadWidgetPosition(),
    getStoredWidgetSize(),
    window.api.invoke('q:store:get', { fontSizePx: 26 })
  ]);

  const widget = document.createElement('div');
  widget.id = 'quran-memorization-widget';

  applyStoredSize(widget, sizeData);
  if (!sizeData.width) widget.style.width = WIDGET_WIDTHS[widgetSize] || '380px';
  if (position) {
    widget.style.left = `${position.left}px`;
    widget.style.top = `${position.top}px`;
    widget.style.bottom = 'auto';
    widget.style.right = 'auto';
  }

  const safePageStats = pageStats || { today: 0, isMemorized: false };
  const readCountText = safePageStats.today > 0
    ? `قرأت ${safePageStats.today} ${safePageStats.today === 1 ? 'مرة' : 'مرات'} اليوم`
    : safePageStats.isMemorized ? 'محفوظة مسبقاً'
      : 'جديدة';

  const fullHeaderHtml = `
    <div class="quran-widget-header" id="quran-header-content">
      <div class="quran-widget-header-top">
        <span class="quran-widget-surah-name">📖 سورة ${surahTitle}</span>
        <span class="quran-widget-page-info">
          <span class="quran-widget-page-number">صفحة ${toArabicNumerals(pageNumber)}</span>
          <span class="quran-widget-read-badge">${readCountText}</span>
        </span>
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
  `;
  const headerContent = hideHeader ? `
    <div class="quran-widget-header-collapsed" id="quran-header-toggle">
      📖 ${surahTitle} — صفحة <span class="collapsed-page-num">${toArabicNumerals(pageNumber)}</span>
    </div>
    ${fullHeaderHtml.replace('id="quran-header-content"', 'id="quran-header-content" style="display:none;"')}
  ` : fullHeaderHtml;

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
    ${headerContent}
    <div class="quran-widget-body">
      ${ayahTextHtml}
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
  const fsPx = fontData.fontSizePx + 'px';
  const bodyEl = widget.querySelector('.quran-widget-body');
  if (bodyEl) bodyEl.style.fontSize = fsPx;
  const nextEl = widget.querySelector('.quran-widget-next-ayah-text');
  if (nextEl) nextEl.style.fontSize = fsPx;

  document.body.appendChild(widget);

  makeWidgetDraggable(widget);
  makeWidgetResizable(widget);

  if (hideHeader) {
    const toggleBtn = document.getElementById('quran-header-toggle');
    const headerEl = document.getElementById('quran-header-content');
    if (toggleBtn && headerEl) {
      toggleBtn.addEventListener('click', () => {
        if (headerEl.style.display === 'none') {
          headerEl.style.display = 'block';
          toggleBtn.style.display = 'none';
        }
      });
    }
  }

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
    resumePausedMedia();
    widget.classList.add('hiding');
    setTimeout(() => {
      cleanupWidget(widget);
      _dismissingWidget = false;
    }, 300);
  });

  document.getElementById('quran-btn-done')?.addEventListener('click', async () => {
    const successMsg = document.getElementById('quran-success-msg');
    successMsg.classList.add('show');

    try {
      let nextPage = pageNumber + 1;
      if (nextPage > 604) nextPage = 1;

      await StorageManager.saveCompletedPage(pageNumber);

      // نرفع الـ flag قبل الحفظ حتى لا يُطفئ storage.onChanged رسالة "ما شاء الله"
      _dismissingWidget = true;
      await window.api.invoke('q:store:set', {
        currentQuranPage: nextPage,
        lastCompletedTime: Date.now()
      });
    } catch (e) {
      console.warn('Quran Widget: done-btn storage error', e);
      _dismissingWidget = true;
    }

    setTimeout(() => {
      resumePausedMedia();
      widget.classList.add('hiding');
      setTimeout(() => {
        cleanupWidget(widget);
        _dismissingWidget = false;
      }, 300);
    }, 2000);
  });
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initQuranWidget);
} else {
  initQuranWidget();
}

window.api.receive('q:store:changed', (changes) => { const areaName = 'local';
  try {
    
    if (areaName === 'local' && changes.lastCompletedTime) {
      if (_dismissingWidget) {
        
        return;
      }

      const widget = document.getElementById('quran-memorization-widget');
      if (widget) {
        resumePausedMedia();
        widget.classList.add('hiding');
        setTimeout(() => cleanupWidget(widget), 300);
      }

      
    }
  } catch (e) {
    console.warn('Quran Widget: storage.onChanged error', e);
  }
});

// ─── setTimeout ذكي بدل setInterval ───



