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

function createChart(): void {
  const canvas = canvasRef.value;
  if (!canvas || chart) {
    return;
  }
  chart = new Chart(canvas, props.config);
}

onMounted(createChart);

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
