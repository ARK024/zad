const StorageManager = {

  // إرجاع تاريخ "اليوم" مع مراعاة وقت بداية اليوم المخصص
  _getEffectiveDate(offsetDays = 0) {
    const now = new Date();
    // نجيب dayStartHour من cache — مش async عشان التواريخ بتتستخدم في أماكن كتير
    // القيمة محملة مسبقاً عبر loadDayStartHour()
    const startHour = StorageManager._dayStartHour || 0;
    // لو الساعة الحالية أقل من وقت البداية، نعتبرها لسه "أمس"
    if (now.getHours() < startHour) {
      now.setDate(now.getDate() - 1);
    }
    now.setDate(now.getDate() + offsetDays);
    return now.toLocaleDateString('en-CA');
  },

  _dayStartHour: 0, // يتحدّث عند التهيئة

  async loadDayStartHour() {
    const data = await window.api.invoke('q:store:get', { dayStartHour: 0 });
    this._dayStartHour = parseInt(data.dayStartHour) || 0;
  },

  getTodayDate() {
    return this._getEffectiveDate(0);
  },

  getYesterday() {
    return this._getEffectiveDate(-1);
  },

  _daysAgoStr(n) {
    return this._getEffectiveDate(-n);
  },

  async initData() {
    const data = await window.api.invoke('q:store:get', [
      'memorizedPages', 'recentReadings', 'totalReadCount',
      'completedPages',
      'dailyStreak', 'lastCompletedDate', 'dailyGoal'
    ]);

    // Migration: لو عنده completedPages قديمة نحوّلها
    if (data.completedPages && data.completedPages.length > 0 && !data.memorizedPages) {
      const uniquePages = [...new Set(data.completedPages.map(p => p.page))].sort((a, b) => a - b);
      const totalReadCount = data.completedPages.length;
      const cutoff = this._daysAgoStr(7);
      const today = this.getTodayDate();
      const recentReadings = data.completedPages
        .filter(p => p.date >= cutoff && p.date <= today)
        .map(p => ({ page: p.page, date: p.date }));

      await window.api.invoke('q:store:set', { memorizedPages: uniquePages, recentReadings, totalReadCount });
      await window.api.invoke('q:store:remove', 'completedPages');
      return;
    }

    if (!data.memorizedPages) {
      await window.api.invoke('q:store:set', {
        memorizedPages: [],
        recentReadings: [],
        totalReadCount: 0,
        dailyStreak: 0,
        lastCompletedDate: null,
        dailyGoal: 1,
        widgetX: null,
        widgetY: null,
      });
    }
  },

  async saveCompletedPage(pageNumber) {
    const today = this.getTodayDate();
    const data = await window.api.invoke('q:store:get', ['memorizedPages', 'recentReadings', 'totalReadCount']);

    // 1. أضف للصفحات المحفوظة لو جديدة
    const memorizedSet = new Set(data.memorizedPages || []);
    memorizedSet.add(pageNumber);
    const memorizedPages = [...memorizedSet].sort((a, b) => a - b);

    // 2. أضف لـ recentReadings وانظّف الأقدم من 7 أيام
    const cutoff = this._daysAgoStr(7);
    const recentReadings = [
      ...(data.recentReadings || []).filter(r => r.date >= cutoff),
      { page: pageNumber, date: today }
    ];

    // 3. زوّد العداد الإجمالي
    const totalReadCount = (data.totalReadCount || 0) + 1;

    await window.api.invoke('q:store:set', { memorizedPages, recentReadings, totalReadCount });
    await this.updateStreak();
  },

  async updateStreak() {
    const data = await window.api.invoke('q:store:get', ['dailyStreak', 'lastCompletedDate']);
    const today = this.getTodayDate();
    const yesterday = this.getYesterday();
    let streak = data.dailyStreak || 0;
    const last = data.lastCompletedDate;

    if (last !== today && last !== yesterday) streak = 0;
    if (last !== today) {
      streak += 1;
      await window.api.invoke('q:store:set', { dailyStreak: streak, lastCompletedDate: today });
    }
  },

  async getDailyProgress() {
    const data = await window.api.invoke('q:store:get', ['dailyGoal', 'recentReadings']);
    const today = this.getTodayDate();
    const dailyGoal = data.dailyGoal || 1;
    const completedToday = (data.recentReadings || []).filter(r => r.date === today).length;
    const percentage = Math.min((completedToday / dailyGoal) * 100, 100);
    return { completed: completedToday, goal: dailyGoal, percentage: Math.round(percentage) };
  },

  async getPageStats(pageNumber) {
    const data = await window.api.invoke('q:store:get', ['recentReadings', 'memorizedPages', 'preloadedPages']);
    const today = this.getTodayDate();
    const todayCount = (data.recentReadings || []).filter(r => r.date === today && r.page === pageNumber).length;
    const isMemorized = (data.memorizedPages || []).includes(pageNumber)
      || (data.preloadedPages || []).includes(pageNumber);
    return { today: todayCount, isMemorized };
  },

  async getTotalStats() {
    const data = await window.api.invoke('q:store:get', ['dailyStreak', 'totalReadCount']);
    const allMemorized = await this.getAllMemorizedPages();
    return {
      uniquePages: allMemorized.length,
      totalReadCount: data.totalReadCount || 0,
      streak: data.dailyStreak || 0,
    };
  },

  async exportData() {
    const data = await window.api.invoke('q:store:get', null);
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `quran-backup-${new Date().toLocaleDateString('en-CA')}.json`;
    a.click();
    URL.revokeObjectURL(url);
  },

  async importData(jsonData) {
    try {
      const data = JSON.parse(jsonData);
      if (!Array.isArray(data.memorizedPages) && !Array.isArray(data.completedPages)) {
        return { success: false, error: 'الملف لا يبدو نسخة احتياطية صحيحة' };
      }
      // فقط نقبل الـ keys المعروفة
      const allowedKeys = [
        'memorizedPages', 'recentReadings', 'totalReadCount', 'completedPages',
        'preloadedPages', 'dailyStreak', 'lastCompletedDate', 'dailyGoal',
        'currentQuranPage', 'memorizationInterval', 'widgetSize', 'hideHeader',
        'reviewEnabled', 'reviewDays', 'recentReviewEnabled', 'fontSizePx',
        'reviewPagesPerSession', 'recentPagesPerSession', 'dayStartHour',
        'reviewIndex', 'lastReviewDate', 'reviewCycleStartDate',
        'reviewSessionStart', 'reviewRetryPages',
        'recentReviewIndex', 'lastRecentReviewDate', 'recentRetryPages',
        'lastCompletedTime', 'pausedUntil', 'testModeEnabled'
      ];
      const sanitized = {};
      for (const key of allowedKeys) {
        if (key in data) sanitized[key] = data[key];
      }
      await window.api.invoke('q:store:set', sanitized);
      await this.initData(); // migration تلقائية لو ملف قديم
      return { success: true };
    } catch (e) {
      return { success: false, error: e.message };
    }
  },

  async resetData() {
    const settings = await window.api.invoke('q:store:get', [
      'currentQuranPage', 'memorizationInterval', 'dailyGoal',
      'widgetSize', 'hideHeader', 'reviewEnabled', 'reviewDays',
      'recentReviewEnabled', 'fontSizePx', 'reviewPagesPerSession',
      'recentPagesPerSession', 'dayStartHour', 'testModeEnabled'
    ]);
    await window.api.invoke('q:store:clear');
    await window.api.invoke('q:store:set', {
      ...settings,
      memorizedPages: [],
      recentReadings: [],
      totalReadCount: 0,
      preloadedPages: [],
      dailyStreak: 0,
      lastCompletedDate: null,
      lastCompletedTime: 0,
      reviewIndex: 0,
      lastReviewDate: null,
      reviewCycleStartDate: null,
      reviewSessionStart: 0,
      reviewRetryPages: [],
      recentReviewIndex: 0,
      lastRecentReviewDate: null,
      recentRetryPages: [],
      pausedUntil: 0,
      widgetX: null,
      widgetY: null,
      widgetCustomWidth: null,
      widgetCustomHeight: null,
      widgetShownAt: 0,
    });
  },

  async saveWidgetPosition(x, y) { await window.api.invoke('q:store:set', { widgetX: x, widgetY: y }); },
  async getWidgetPosition() {
    const data = await window.api.invoke('q:store:get', ['widgetX', 'widgetY']);
    return { x: data.widgetX, y: data.widgetY };
  },
  async saveWidgetSize(width, height) { await window.api.invoke('q:store:set', { widgetCustomWidth: width, widgetCustomHeight: height }); },
  async getWidgetSize() {
    const data = await window.api.invoke('q:store:get', ['widgetCustomWidth', 'widgetCustomHeight']);
    return { width: data.widgetCustomWidth || null, height: data.widgetCustomHeight || null };
  },

  // إعدادات عدد الصفحات في جلسة المراجعة
  async setReviewPagesPerSession(count) {
    const safeCount = Math.max(1, Math.min(50, parseInt(count) || 10));  // بين 1 و 50
    await window.api.invoke('q:store:set', { reviewPagesPerSession: safeCount });
    return safeCount;
  },

  async getReviewPagesPerSession() {
    const data = await window.api.invoke('q:store:get', ['reviewPagesPerSession']);
    return parseInt(data.reviewPagesPerSession) || 10;  // افتراضي: 10
  },

  async getDayIndexInCycle(reviewDays) {
    const stored = await window.api.invoke('q:store:get', ['reviewCycleStartDate']);
    const today = this.getTodayDate();
    if (stored.reviewCycleStartDate) {
      const [sy, sm, sd] = stored.reviewCycleStartDate.split('-').map(Number);
      const [ty, tm, td] = today.split('-').map(Number);
      const diffDays = Math.floor((Date.UTC(ty, tm - 1, td) - Date.UTC(sy, sm - 1, sd)) / 86400000);
      return diffDays % reviewDays;
    }
    await window.api.invoke('q:store:set', { reviewCycleStartDate: today });
    return 0;
  },

  async getReviewSettings() {
    const data = await window.api.invoke('q:store:get', ['reviewEnabled', 'reviewDays', 'reviewIndex', 'lastReviewDate', 'reviewPagesPerSession']);
    return {
      enabled: data.reviewEnabled || false,
      days: data.reviewDays || 7,
      reviewIndex: data.reviewIndex || 0,
      lastReviewDate: data.lastReviewDate || null,
      pagesPerSession: parseInt(data.reviewPagesPerSession) || 10,  // افتراضي: 10 صفحات في الجلسة
    };
  },

  async getTodayReviewPages() {
    await this.loadDayStartHour();
    const settings = await this.getReviewSettings();
    const memorizedPages = await this.getAllMemorizedPages();
    if (memorizedPages.length === 0) {
      return { pages: [], currentIndex: 0, totalToday: 0 };
    }

    const dayIndex = await this.getDayIndexInCycle(settings.days);

    // نشيل الصفحات القريبة الأول، وبعدين نقسم الباقي على الأيام
    const recentPages = await this.getRecentPages();
    const recentSet = new Set(recentPages);
    const nonRecentPages = memorizedPages.filter(p => !recentSet.has(p));

    const days = settings.days;
    const today = this.getTodayDate();
    let currentIndex = settings.reviewIndex;
    const sessionData = await window.api.invoke('q:store:get', {
      reviewPagesPerSession: 0,
      reviewSessionStart: 0,
      reviewRetryPages: []
    });
    let sessionStart = sessionData.reviewSessionStart || 0;
    let effectiveRetryPages = sessionData.reviewRetryPages || [];

    if (settings.lastReviewDate !== today) {
      currentIndex = 0; sessionStart = 0;
      effectiveRetryPages = [];
      await window.api.invoke('q:store:set', {
        reviewIndex: 0, lastReviewDate: today,
        reviewSessionStart: 0, reviewRetryPages: []
      });
    }

    // نقسم الصفحات غير القريبة على الأيام بالتساوي
    const n = nonRecentPages.length;
    const base = Math.floor(n / days);
    const extra = n % days;

    const startIdx = dayIndex < extra
      ? dayIndex * (base + 1)
      : extra * (base + 1) + (dayIndex - extra) * base;
    const count = dayIndex < extra ? base + 1 : base;
    const endIdx = startIdx + count;

    const basePages = nonRecentPages.slice(startIdx, endIdx);
    const retryPages = effectiveRetryPages;
    const todayPages = [...basePages, ...retryPages];

    // حساب الجلسات بناءً على reviewPagesPerSession
    const pagesPerSession = sessionData.reviewPagesPerSession || 10;  // افتراضي: 10
    const totalSessions = pagesPerSession > 0 ? Math.ceil(todayPages.length / pagesPerSession) : 1;
    const currentSession = pagesPerSession > 0 ? Math.floor(currentIndex / pagesPerSession) : 0;
    
    // تحديد الصفحات اللي في الجلسة الحالية
    sessionStart = currentSession * pagesPerSession;
    const sessionEnd = Math.min(sessionStart + pagesPerSession, todayPages.length);
    const sessionPages = todayPages.slice(sessionStart, sessionEnd);
    
    //currentIndex داخل الجلسة
    const indexInSession = currentIndex % pagesPerSession;

    return {
      pages: sessionPages,  // صفحات الجلسة الحالية فقط
      allPages: todayPages,  // كل صفحات اليوم (للمرجعية)
      currentIndex: indexInSession,  // الاندكس داخل الجلسة
      globalIndex: currentIndex,  // الاندكس العام
      totalToday: todayPages.length,  // إجمالي صفحات اليوم
      currentSession: currentSession + 1,  // رقم الجلسة الحالية
      totalSessions: totalSessions,  // إجمالي الجلسات
      pagesPerSession: pagesPerSession,  // عدد الصفحات في الجلسة
      dayIndex: dayIndex + 1, 
      totalDays: settings.days,
      sessionStart,
      sessionEnd
    };
  },

  async incrementReviewIndex() {
    const data = await window.api.invoke('q:store:get', ['reviewIndex']);
    const newIndex = (data.reviewIndex || 0) + 1;
    await window.api.invoke('q:store:set', { reviewIndex: newIndex });
    return newIndex;
  },

  // التنقل بين الجلسات
  async nextSession() {
    const data = await window.api.invoke('q:store:get', ['reviewIndex', 'reviewPagesPerSession']);
    const pagesPerSession = parseInt(data.reviewPagesPerSession) || 10;
    const newIndex = (data.reviewIndex || 0) + pagesPerSession;
    await window.api.invoke('q:store:set', { reviewIndex: newIndex });
    return newIndex;
  },

  async prevSession() {
    const data = await window.api.invoke('q:store:get', ['reviewIndex', 'reviewPagesPerSession']);
    const pagesPerSession = parseInt(data.reviewPagesPerSession) || 10;
    const newIndex = Math.max(0, (data.reviewIndex || 0) - pagesPerSession);
    await window.api.invoke('q:store:set', { reviewIndex: newIndex });
    return newIndex;
  },

  async advanceReviewSession(newSessionStart) {
    await window.api.invoke('q:store:set', { reviewSessionStart: newSessionStart });
  },

  // "إعادة" — يعيد نفس الصفحة فوراً بدون تقدم للتالية
  async retryReviewPage() {
    const reviewData = await this.getTodayReviewPages();
    const pageToRetry = reviewData.pages[reviewData.currentIndex];
    
    // نحفظ إن الصفحة دي ضعيفة ومحتاج إعادة
    const data = await window.api.invoke('q:store:get', { weakPages: [] });
    const weakPages = data.weakPages || [];
    
    if (pageToRetry !== undefined && !weakPages.includes(pageToRetry)) {
      weakPages.push(pageToRetry);
      await window.api.invoke('q:store:set', { weakPages: weakPages });
    }
    
    // ❌ مش بنتقدم في الـindex! الصفحة هتتعرض تاني
    // المستخدم هو اللي هيضغط "التالي" لما يخلص
    
    return pageToRetry;
  },

  // "إعادة" للمراجعات القريبة — نفس المبدأ بدون تقدم
  async retryRecentReviewPage() {
    const reviewData = await this.getTodayRecentReviewData();
    const pageToRetry = reviewData.pages[reviewData.currentIndex];
    
    // نحفظ إن الصفحة دي ضعيفة
    const data = await window.api.invoke('q:store:get', { weakPages: [] });
    const weakPages = data.weakPages || [];
    
    if (pageToRetry !== undefined && !weakPages.includes(pageToRetry)) {
      weakPages.push(pageToRetry);
      await window.api.invoke('q:store:set', { weakPages: weakPages });
    }
    
    // ❌ مش بنتقدم في الـindex!
    return pageToRetry;
  },

  async getRecentPages() {
    const data = await window.api.invoke('q:store:get', ['recentReadings']);
    const today = this.getTodayDate();
    const cutoff = this._daysAgoStr(7);
    const recentSet = new Set();
    (data.recentReadings || []).forEach(r => {
      if (r.date >= cutoff && r.date < today) recentSet.add(r.page);
    });
    return [...recentSet].sort((a, b) => a - b);
  },

  async getTodayRecentReviewData() {
    await this.loadDayStartHour();
    const recentPages = await this.getRecentPages();
    if (recentPages.length === 0) return { pages: [], currentIndex: 0, totalToday: 0, enabled: false };

    const today = this.getTodayDate();
    const data = await window.api.invoke('q:store:get', {
      recentReviewIndex: 0,
      lastRecentReviewDate: null,
      recentPagesPerSession: 0,
      recentRetryPages: []
    });
    let currentIndex = data.recentReviewIndex || 0;
    let effectiveRetryPages = data.recentRetryPages || [];

    if (data.lastRecentReviewDate !== today) {
      currentIndex = 0;
      effectiveRetryPages = []; // ← لا نستخدم القيمة القديمة من قبل الـ reset
      await window.api.invoke('q:store:set', {
        recentReviewIndex: 0,
        lastRecentReviewDate: today,
        recentRetryPages: []
      });
    }

    const retryPages = effectiveRetryPages.filter(p => recentPages.includes(p));
    const basePages = recentPages.filter(p => !retryPages.includes(p));
    const allPages = [...basePages, ...retryPages];

    return {
      pages: allPages,
      currentIndex,
      totalToday: allPages.length,
      pagesPerSession: parseInt(data.recentPagesPerSession) || 0,
      enabled: true
    };
  },

  async incrementRecentReviewIndex(count = 1) {
    const data = await window.api.invoke('q:store:get', ['recentReviewIndex']);
    const newIndex = (data.recentReviewIndex || 0) + count;
    await window.api.invoke('q:store:set', { recentReviewIndex: newIndex });
    return newIndex;
  },

  // "تخطي" — يتقدم للتالية بدون إعادة
  async skipRecentReviewPage() {
    const data = await window.api.invoke('q:store:get', ['recentReviewIndex']);
    const newIndex = (data.recentReviewIndex || 0) + 1;
    await window.api.invoke('q:store:set', { recentReviewIndex: newIndex });
    return newIndex;
  },

  async getMemorizedPages() {
    const data = await window.api.invoke('q:store:get', ['memorizedPages']);
    return (data.memorizedPages || []).sort((a, b) => a - b);
  },

  async getPreloadedPages() {
    const data = await window.api.invoke('q:store:get', ['preloadedPages']);
    return data.preloadedPages || [];
  },

  async addPreloadedRange(from, to) {
    const existing = await this.getPreloadedPages();
    const s = new Set(existing);
    for (let p = from; p <= to; p++) s.add(p);
    const sorted = [...s].sort((a, b) => a - b);
    await window.api.invoke('q:store:set', { preloadedPages: sorted });
    return sorted.length;
  },

  async addPreloadedPages(pagesArray) {
    const existing = await this.getPreloadedPages();
    const merged = new Set([...existing, ...pagesArray.filter(p => p >= 1 && p <= 604)]);
    const sorted = [...merged].sort((a, b) => a - b);
    await window.api.invoke('q:store:set', { preloadedPages: sorted });
    return sorted.length;
  },

  async removePreloadedPages(pagesArray) {
    const existing = await this.getPreloadedPages();
    const removeSet = new Set(pagesArray);
    await window.api.invoke('q:store:set', { preloadedPages: existing.filter(p => !removeSet.has(p)) });
  },

  async removePreloadedRange(from, to) {
    const existing = await this.getPreloadedPages();
    await window.api.invoke('q:store:set', { preloadedPages: existing.filter(p => p < from || p > to) });
  },

  async getAllMemorizedPages() {
    const [memorized, preloaded] = await Promise.all([this.getMemorizedPages(), this.getPreloadedPages()]);
    return [...new Set([...memorized, ...preloaded])].sort((a, b) => a - b);
  }
};
