import { onBeforeUnmount, onMounted } from "vue";

interface ScrollbarAnimation {
  timer: number;
  frame: number;
}

/**
 * 捕获页面内所有滚动事件并独立维护各滚动区域的滚动条可见性：
 * 滚动时立即置为不透明，闲置 800ms 后用 250ms 线性淡出；
 * prefers-reduced-motion 时直接归零。
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
      // 滚动条伪元素不能稳定支持 CSS 过渡，改用逐帧写入变量。
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
