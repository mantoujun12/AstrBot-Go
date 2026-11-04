<script setup lang="ts" generic="TType extends ChartType">
import {
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  Chart,
  DoughnutController,
  Filler,
  Legend,
  LineController,
  LineElement,
  LinearScale,
  PointElement,
  Tooltip,
  type ChartConfiguration,
  type ChartType,
} from "chart.js";
import { onBeforeUnmount, onMounted, ref, watch } from "vue";

Chart.register(
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  DoughnutController,
  Filler,
  Legend,
  LineController,
  LineElement,
  LinearScale,
  PointElement,
  Tooltip,
);

const props = defineProps<{
  config: ChartConfiguration<TType>;
  height: number;
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
let chart: Chart<TType> | null = null;
let observer: ResizeObserver | null = null;

function chartContainer(): HTMLElement | null {
  return canvasRef.value?.parentElement ?? null;
}

function hasLayoutSize(element: HTMLElement | null): boolean {
  return element !== null && element.clientWidth > 0 && element.clientHeight > 0;
}

function createChart(): void {
  const canvas = canvasRef.value;
  if (!canvas || chart || !hasLayoutSize(canvas.parentElement)) {
    return;
  }
  chart = new Chart(canvas, props.config);
}

onMounted(() => {
  const container = chartContainer();
  if (!container) {
    return;
  }
  // Dialog content is laid out after it opens, so a chart created too early
  // ends up with a zero-sized canvas. Build it only once the box has a size.
  observer = new ResizeObserver(() => {
    if (chart) {
      chart.resize();
      return;
    }
    createChart();
  });
  observer.observe(container);
  createChart();
});

watch(
  () => props.config,
  (config) => {
    if (!chart) {
      createChart();
      return;
    }
    chart.data = config.data;
    if (config.options) {
      chart.options = config.options;
    }
    chart.update();
  },
);

watch(
  () => props.height,
  () => {
    chart?.resize();
  },
  { flush: "post" },
);

onBeforeUnmount(() => {
  observer?.disconnect();
  observer = null;
  chart?.destroy();
  chart = null;
});
</script>

<template>
  <div class="canvas-chart" :style="{ height: `${height}px` }">
    <canvas ref="canvasRef" aria-hidden="true" />
  </div>
</template>

<style scoped>
.canvas-chart {
  position: relative;
  width: 100%;
  min-width: 0;
}

.canvas-chart canvas {
  display: block;
  width: 100%;
  height: 100%;
}
</style>
