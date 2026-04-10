/**
 * CricketBrain — Main Application JavaScript
 *
 * Handles: theme switching, scroll animations, SPA-like page loading,
 * animated counters, copy-code buttons, mobile nav, scroll progress,
 * smooth scroll, skip-nav, and optional live GitHub data.
 *
 * No frameworks. No dependencies. ES2022+.
 */

// ---------------------------------------------------------------------------
// 1. THEME TOGGLE
// ---------------------------------------------------------------------------

const ThemeManager = (() => {
  const STORAGE_KEY = 'theme';
  const DARK = 'dark';
  const LIGHT = 'light';

  /** Sun SVG icon (shown when theme is dark → click to go light). */
  const SUN_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`;

  /** Moon SVG icon (shown when theme is light → click to go dark). */
  const MOON_ICON = `<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`;

  function getPreferred() {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === DARK || stored === LIGHT) return stored;
    return window.matchMedia?.('(prefers-color-scheme: light)').matches
      ? LIGHT
      : DARK;
  }

  function apply(theme) {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem(STORAGE_KEY, theme);
    const btn = document.getElementById('theme-toggle');
    if (btn) {
      btn.innerHTML = theme === DARK ? SUN_ICON : MOON_ICON;
      btn.setAttribute('aria-label', `Switch to ${theme === DARK ? 'light' : 'dark'} theme`);
    }
  }

  function toggle() {
    const next = document.documentElement.getAttribute('data-theme') === DARK
      ? LIGHT
      : DARK;

    // Use View Transition API when available for a smooth crossfade.
    if (document.startViewTransition) {
      document.startViewTransition(() => apply(next));
    } else {
      apply(next);
    }
  }

  function init() {
    apply(getPreferred());
    document.getElementById('theme-toggle')?.addEventListener('click', toggle);
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 2. SCROLL ANIMATIONS (IntersectionObserver)
// ---------------------------------------------------------------------------

const ScrollAnimations = (() => {
  /** Respect user motion preferences. */
  function prefersReducedMotion() {
    return window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  }

  function init() {
    if (prefersReducedMotion()) {
      // Immediately reveal everything without animation.
      document.querySelectorAll('[data-animate]').forEach((el) => {
        el.classList.add('animate-in');
      });
      return;
    }

    const observer = new IntersectionObserver(
      (entries, obs) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) return;

          const el = entry.target;

          if (el.dataset.stagger === 'true') {
            // Stagger each child by 100 ms.
            Array.from(el.children).forEach((child, i) => {
              child.style.transitionDelay = `${i * 100}ms`;
              child.classList.add('animate-in');
            });
          }

          el.classList.add('animate-in');
          obs.unobserve(el);
        });
      },
      { threshold: 0.1 },
    );

    document.querySelectorAll('[data-animate]').forEach((el) => observer.observe(el));
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 3. ANIMATED COUNTERS
// ---------------------------------------------------------------------------

const AnimatedCounters = (() => {
  /** Ease-out cubic curve. */
  function easeOut(t) {
    return 1 - (1 - t) ** 3;
  }

  function animateCounter(el) {
    const target = parseInt(el.dataset.countTo, 10);
    if (Number.isNaN(target)) return;

    const duration = 2000; // ms
    const start = performance.now();

    function step(now) {
      const progress = Math.min((now - start) / duration, 1);
      const value = Math.round(easeOut(progress) * target);
      el.textContent = value.toLocaleString();
      if (progress < 1) requestAnimationFrame(step);
    }

    requestAnimationFrame(step);
  }

  function init() {
    const elements = document.querySelectorAll('[data-count-to]');
    if (elements.length === 0) return;

    const observer = new IntersectionObserver(
      (entries, obs) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) return;
          animateCounter(entry.target);
          obs.unobserve(entry.target);
        });
      },
      { threshold: 0.1 },
    );

    elements.forEach((el) => {
      el.textContent = '0'; // Start at zero.
      observer.observe(el);
    });
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 4. COPY CODE BUTTONS
// ---------------------------------------------------------------------------

const CopyCodeButtons = (() => {
  function addButton(block) {
    // Avoid duplicating if already present.
    if (block.querySelector('.copy-btn')) return;

    const btn = document.createElement('button');
    btn.className = 'copy-btn';
    btn.setAttribute('aria-label', 'Copy code');
    btn.textContent = 'Copy';

    btn.addEventListener('click', async () => {
      try {
        const code = block.querySelector('code, pre')?.textContent ?? block.textContent;
        await navigator.clipboard.writeText(code);
        btn.textContent = 'Copied!';
        setTimeout(() => {
          btn.textContent = 'Copy';
        }, 2000);
      } catch {
        // Clipboard API may fail in insecure contexts — silent fallback.
        btn.textContent = 'Error';
        setTimeout(() => {
          btn.textContent = 'Copy';
        }, 2000);
      }
    });

    block.style.position ??= 'relative';
    block.appendChild(btn);
  }

  function init() {
    document.querySelectorAll('.code-block').forEach(addButton);
  }

  return { init, addButton };
})();

// ---------------------------------------------------------------------------
// 5. SMOOTH SCROLL & ACTIVE NAV HIGHLIGHTING
// ---------------------------------------------------------------------------

const SmoothScroll = (() => {
  function init() {
    // Intercept anchor links for smooth scrolling.
    document.addEventListener('click', (e) => {
      const link = e.target.closest('a[href^="#"]');
      if (!link) return;

      const id = link.getAttribute('href')?.slice(1);
      if (!id) return;

      const target = document.getElementById(id);
      if (!target) return;

      e.preventDefault();
      target.scrollIntoView({ behavior: 'smooth' });
      history.replaceState(null, '', `#${id}`);
    });

    // Highlight active nav link based on visible section.
    const sections = document.querySelectorAll('section[id]');
    const navLinks = document.querySelectorAll('nav a[href^="#"]');
    if (sections.length === 0 || navLinks.length === 0) return;

    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) return;
          const id = entry.target.id;
          navLinks.forEach((link) => {
            link.classList.toggle('active', link.getAttribute('href') === `#${id}`);
          });
        });
      },
      { rootMargin: '-20% 0px -60% 0px' },
    );

    sections.forEach((section) => observer.observe(section));
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 6. MOBILE NAV
// ---------------------------------------------------------------------------

const MobileNav = (() => {
  // Closure-scoped private state.
  let headerEl = null;
  let toggleBtn = null;

  function isOpen() {
    return headerEl?.classList.contains('nav-open') ?? false;
  }

  function open() {
    headerEl?.classList.add('nav-open');
    toggleBtn?.setAttribute('aria-expanded', 'true');
    trapFocus();
  }

  function close() {
    headerEl?.classList.remove('nav-open');
    toggleBtn?.setAttribute('aria-expanded', 'false');
    toggleBtn?.focus();
  }

  function toggle() {
    isOpen() ? close() : open();
  }

  /** Basic focus trap: keep Tab cycling inside the nav while open. */
  function trapFocus() {
    const focusable = headerEl?.querySelectorAll(
      'a[href], button, input, textarea, select, [tabindex]:not([tabindex="-1"])',
    );
    if (!focusable?.length) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    headerEl?.addEventListener('keydown', function handler(e) {
      if (!isOpen()) {
        headerEl.removeEventListener('keydown', handler);
        return;
      }
      if (e.key !== 'Tab') return;

      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    });

    first?.focus();
  }

  function init() {
    headerEl = document.querySelector('header');
    toggleBtn = document.getElementById('nav-toggle');
    if (!headerEl || !toggleBtn) return;

    toggleBtn.addEventListener('click', toggle);

    // Close on outside click.
    document.addEventListener('click', (e) => {
      if (isOpen() && !headerEl.contains(e.target)) close();
    });

    // Close on Escape.
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && isOpen()) close();
    });
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 6b. NAV DROPDOWN (Use Cases submenu)
// ---------------------------------------------------------------------------

const NavDropdown = (() => {
  function init() {
    document.querySelectorAll('.nav-dropdown-toggle').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        const dropdown = btn.closest('.nav-dropdown');
        if (!dropdown) return;
        const isOpen = dropdown.classList.contains('open');
        // Close all dropdowns first
        document.querySelectorAll('.nav-dropdown.open').forEach((d) => d.classList.remove('open'));
        if (!isOpen) {
          dropdown.classList.add('open');
          btn.setAttribute('aria-expanded', 'true');
        } else {
          btn.setAttribute('aria-expanded', 'false');
        }
      });
    });

    // Close dropdown on outside click
    document.addEventListener('click', (e) => {
      if (!e.target.closest('.nav-dropdown')) {
        document.querySelectorAll('.nav-dropdown.open').forEach((d) => {
          d.classList.remove('open');
          d.querySelector('.nav-dropdown-toggle')?.setAttribute('aria-expanded', 'false');
        });
      }
    });

    // Close dropdown on Escape
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        document.querySelectorAll('.nav-dropdown.open').forEach((d) => {
          d.classList.remove('open');
          d.querySelector('.nav-dropdown-toggle')?.setAttribute('aria-expanded', 'false');
        });
      }
    });
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 7. SPA-LIKE PAGE LOADING (Legal Pages)
// ---------------------------------------------------------------------------

const SPARouter = (() => {
  /** Cache fetched page content keyed by URL path. */
  const cache = new Map();

  /** Store the original index page <main> content on first navigation. */
  let indexContent = null;
  let indexTitle = '';

  /** Routes we handle as SPA navigations.
   *  Only pages that use NO custom <style> in <head> — they share the main CSS.
   *  Pages with their own styles (whitepaper, cardiac) must load as full pages. */
  const SPA_ROUTES = ['pages/impressum.html', 'pages/datenschutz.html'];

  function isSPALink(href) {
    if (!href) return false;
    try {
      const url = new URL(href, location.origin);
      return SPA_ROUTES.some((r) => url.pathname.endsWith(r));
    } catch {
      return false;
    }
  }

  async function loadPage(url, pushState = true) {
    const mainEl = document.querySelector('main');
    if (!mainEl) return fallback(url);

    // Save index content on first SPA navigation.
    if (indexContent === null) {
      indexContent = mainEl.innerHTML;
      indexTitle = document.title;
    }

    // Check cache.
    let html = cache.get(url);

    if (!html) {
      try {
        const resp = await fetch(url);
        if (!resp.ok) return fallback(url);
        const text = await resp.text();

        // Extract <main> content from the fetched document.
        const parser = new DOMParser();
        const doc = parser.parseFromString(text, 'text/html');
        const fetchedMain = doc.querySelector('main');
        if (!fetchedMain) return fallback(url);

        html = {
          content: fetchedMain.innerHTML,
          title: doc.title || document.title,
        };
        cache.set(url, html);
      } catch {
        return fallback(url);
      }
    }

    const swap = () => {
      mainEl.innerHTML = html.content;
      document.title = html.title;
      window.scrollTo({ top: 0, behavior: 'smooth' });

      // Re-initialize dynamic features for new content.
      CopyCodeButtons.init();
      ScrollAnimations.init();
      AnimatedCounters.init();
    };

    if (document.startViewTransition) {
      document.startViewTransition(swap);
    } else {
      swap();
    }

    if (pushState) {
      history.pushState({ spa: true, url }, html.title, url);
    }
  }

  function restoreIndex() {
    const mainEl = document.querySelector('main');
    if (!mainEl || indexContent === null) return;

    const swap = () => {
      mainEl.innerHTML = indexContent;
      document.title = indexTitle;
      window.scrollTo({ top: 0, behavior: 'smooth' });

      // Re-initialize dynamic features.
      CopyCodeButtons.init();
      ScrollAnimations.init();
      AnimatedCounters.init();
    };

    if (document.startViewTransition) {
      document.startViewTransition(swap);
    } else {
      swap();
    }
  }

  function fallback(url) {
    location.href = url;
  }

  function init() {
    // Intercept clicks on SPA-eligible links.
    document.addEventListener('click', (e) => {
      const link = e.target.closest('a');
      if (!link) return;

      const href = link.getAttribute('href');
      if (isSPALink(link.href)) {
        e.preventDefault();
        loadPage(href);
      }
    });

    // Handle browser back/forward.
    window.addEventListener('popstate', (e) => {
      if (e.state?.spa) {
        loadPage(e.state.url, false);
      } else if (indexContent !== null) {
        restoreIndex();
      }
    });
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 8. SKIP NAV
// ---------------------------------------------------------------------------

const SkipNav = (() => {
  function init() {
    const skipLink = document.getElementById('skip-nav');
    if (!skipLink) return;

    skipLink.addEventListener('click', (e) => {
      e.preventDefault();
      const target = document.getElementById('main-content');
      if (!target) return;
      target.setAttribute('tabindex', '-1');
      target.focus();
    });
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 9. SCROLL PROGRESS BAR
// ---------------------------------------------------------------------------

const ScrollProgress = (() => {
  let bar = null;

  function createBar() {
    bar = document.createElement('div');
    bar.id = 'scroll-progress';
    Object.assign(bar.style, {
      position: 'fixed',
      top: '0',
      left: '0',
      height: '3px',
      width: '0%',
      background: 'var(--accent, #00bfa6)',
      zIndex: '9999',
      transition: 'width 50ms linear',
      pointerEvents: 'none',
    });
    document.body.prepend(bar);
  }

  function update() {
    if (!bar) return;
    const scrollable = document.documentElement.scrollHeight - window.innerHeight;
    const pct = scrollable > 0 ? (window.scrollY / scrollable) * 100 : 0;
    bar.style.width = `${pct}%`;
  }

  function init() {
    createBar();
    window.addEventListener('scroll', update, { passive: true });
    update();
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// 10. LIVE GITHUB DATA (optional, graceful fallback)
// ---------------------------------------------------------------------------

const GitHubData = (() => {
  const REPO_API = 'https://api.github.com/repos/BEKO2210/cricket-brain';
  let fetched = false;

  async function init() {
    if (fetched) return;
    fetched = true;

    try {
      const resp = await fetch(REPO_API, {
        headers: { Accept: 'application/vnd.github.v3+json' },
      });
      if (!resp.ok) return;

      const data = await resp.json();
      const stars = data.stargazers_count;
      const updatedAt = data.pushed_at ?? data.updated_at;

      // Populate star count element if present.
      const starEl = document.querySelector('[data-github="stars"]');
      if (starEl && typeof stars === 'number') {
        starEl.textContent = stars.toLocaleString();
      }

      // Populate last-updated element if present.
      const updatedEl = document.querySelector('[data-github="updated"]');
      if (updatedEl && updatedAt) {
        const date = new Date(updatedAt);
        updatedEl.textContent = date.toLocaleDateString(undefined, {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
        });
      }
    } catch {
      // Silent failure — no error shown to user.
    }
  }

  return { init };
})();

// ---------------------------------------------------------------------------
// BOOTSTRAP
// ---------------------------------------------------------------------------

document.addEventListener('DOMContentLoaded', () => {
  ThemeManager.init();
  ScrollAnimations.init();
  AnimatedCounters.init();
  CopyCodeButtons.init();
  SmoothScroll.init();
  MobileNav.init();
  NavDropdown.init();
  SPARouter.init();
  SkipNav.init();
  ScrollProgress.init();
  GitHubData.init();
});
