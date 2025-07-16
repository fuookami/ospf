<template>
  <v-container ref="container" fluid style="margin: 0; padding: 0;">
    <v-container ref="toolbar" fluid class="d-flex flex-row-reverse" style="margin: 0; padding: 0;">
      <v-btn density="compact" icon="mdi-plus" style="margin-right: .5em;" @click="rescale(1)" />
      <v-btn density="compact" icon="mdi-minus" style="margin-right: .5em;" @click="rescale(-1)" />
      <v-select v-model="visibleLines" :items="lines" label="可见行" multiple="true">
        <template v-slot:prepend-item>
          <v-list-item title="选择全部" @click="setAllLineVisible" />
          <v-list-item title="选择关联行" @click="setLinkedLineVisible" />
        </template>
      </v-select>
    </v-container>
    <gantt-chart-view ref="ganttChart" :style="{ height: height + 'px' }" @focus="focus" />
  </v-container>
</template>

<script lang="ts">
import {defineComponent, ref, watch} from "vue";
import GanttChartView from "./gantt-chart/chart.vue"
import { GanttChartVO, GanttLineVO } from "./vo.ts";

function selectedLinkedLines(lines: Array<GanttLineVO>, linkedInfo: Map<string, string>) {
  const ret = [];
  for (const line of lines) {
    for (const [linkedKey, linkedValue] of linkedInfo.entries()) {
      if (line.items.find((item) => Array.from(item.info.entries()).find(([key, value]) => key === linkedKey && value === linkedValue))) {
        ret.push(line.name);
        break
      }
    }
  }
  return ret;
}

export default defineComponent({
  name: "GanttChart",

  components: {
    GanttChartView
  },

  setup() {
    const container = ref<HTMLElement>();
    const toolbar = ref<HTMLElement>();
    const ganttChart = ref<typeof GanttChartView | null>();

    const schema = ref<GanttChartVO>();
    const height = ref(0);
    const minScale = ref(0);
    const maxScale = ref(0);
    const scale = ref(0);
    const lines = ref<Array<String>>([]);
    const visibleLines = ref<Array<String>>([]);
    const linkedLines = ref<Array<String>>([]);

    function render(data: GanttChartVO) {
      schema.value = data;
      height.value = container.value!!.$el.offsetHeight - toolbar.value!!.$el.offsetHeight;
      ganttChart.value!!.init(data, container.value!!.$el.offsetWidth, height.value);
      minScale.value = 0 - Math.floor(ganttChart.value!!.scales.length / 2);
      maxScale.value = minScale.value + ganttChart.value!!.scales.length - 1;
      scale.value = 0;
      lines.value = schema.value!!.lines.map(line => line.name);
      visibleLines.value = schema.value!!.lines.map(line => line.name);
    }

    function resize(newWidth: number, newHeight: number) {
      if (ganttChart.value) {
        height.value = newHeight - toolbar.value!!.offsetHeight;
        ganttChart.value.resize(newWidth, height.value);
      }
    }

    function rescale(diff: number) {
      let newScale = scale.value + diff;
      if (newScale > maxScale.value) {
        newScale = maxScale.value;
      }
      if (newScale < minScale.value) {
        newScale = minScale.value;
      }
      if (newScale !== scale.value) {
        ganttChart.value!!.rescale(scale.value - minScale.value, newScale - minScale.value);
      }
      scale.value += diff;
    }

    function focus(linkedInfo: Map<string, string>) {
      linkedLines.value = selectedLinkedLines(schema.value!!.lines, linkedInfo);
    }

    function setAllLineVisible() {
      visibleLines.value = schema.value!!.lines.map(line => line.name);
      ganttChart.value!!.setVisibleLines(schema.value!!.lines.map(line => line.name));
    }

    function setLinkedLineVisible() {
      visibleLines.value = linkedLines.value;
      ganttChart.value!!.setVisibleLines(linkedLines.value);
    }

    watch(visibleLines, (newVisibleLines): void => {
      ganttChart.value!!.setVisibleLines(newVisibleLines);
    });

    return {
      container,
      toolbar,
      ganttChart,
      schema,
      height,
      minScale,
      maxScale,
      scale,
      lines,
      visibleLines,
      linkedLines,
      render,
      resize,
      rescale,
      focus,
      setAllLineVisible,
      setLinkedLineVisible
    }
  }
});

</script>
