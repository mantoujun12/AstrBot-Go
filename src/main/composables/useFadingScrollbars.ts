import { onBeforeUnmount, onMounted } from "vue";

interface ScrollbarAnimation {
  timer: number;
  frame: number;
}

/**
 * Tracks scroll events across the page and manages each scroll area's
 * scrollbar visibility independently: opaque while scrolling, fades out over
 * 250ms after 800ms idle; drops straight to zero under prefers-reduced-motion.
 */
export function useFadingScrollbars(): void {
  const animations = new Map<HTMLElement, ScrollbarAnimation>();
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

  const onScrollbarScroll = (event: Event): void => {
    const element =
      event.target === document ? document.scrollingElement : event.target;
    if (!(element instanceof HTMLElement)) return;

    const previous = animations.get(element);
    if (previous) {
      window.clearTimeout(previous.timer);
      window.cancelAnimationFrame(previous.frame);
    }
    element.style.setProperty("--astrbot-scrollbar-opacity", "1");
    const animation: ScrollbarAnimation = { timer: 0, frame: 0 };
    animation.timer = window.setTimeout(() => {
      const started = performance.now();
      // Scrollbar pseudo-elements can't reliably run CSS transitions, so the variable is written per frame.
      const fadeScrollbar = (now: number): void => {
        const opacity = reducedMotion.matches
          ? 0
          : Math.max(0, Math.min(1, 1 - (now - started) / 250));
        element.style.setProperty(
          "--astrbot-scrollbar-opacity",
          String(opacity),
        );
        if (opacity > 0) {
          animation.frame = window.requestAnimationFrame(fadeScrollbar);
        } else {
          element.style.removeProperty("--astrbot-scrollbar-opacity");
          animations.delete(element);
        }
      };
      animation.frame = window.requestAnimationFrame(fadeScrollbar);
    }, 800);
    animations.set(element, animation);
  };

  onMounted(() => {
    document.addEventListener("scroll", onScrollbarScroll, {
      capture: true,
      passive: true,
    });
  });

  onBeforeUnmount(() => {
    document.removeEventListener("scroll", onScrollbarScroll, {
      capture: true,
    });
    for (const [element, animation] of animations) {
      window.clearTimeout(animation.timer);
      window.cancelAnimationFrame(animation.frame);
      element.style.removeProperty("--astrbot-scrollbar-opacity");
    }
    animations.clear();
  });
}
