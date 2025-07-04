<template>
  <v-container 
    :style="{ width: width + 'px', height: height + 'em' }"
    style=" margin: 0; padding: 0; position: relative;
  ">
    <gantt-bar v-for="(item, _) of items" 
      :ref="(el) => setViewRefs(el, item)"
      :style="{ left: item.x + 'px', top: item.y + 'em' }" 
      style="position: absolute;"
      @focused="focused"
    />
  </v-container>
</template>

<script lang="ts">
import "./chart.css"
import { ComponentPublicInstance, HTMLAttributes, defineComponent, defineEmits, ref } from "vue";
import dayjs from "dayjs";
import GanttBar from "./bar.vue";
import { GanttItem } from "../dto.ts";

const baseLineHeight = 3.5;

type GanttItemView = {
  x: number
  y: number
  item: GanttItem
  view: typeof GanttBar | null
}

function calculateHeights(startTime: dayjs.Dayjs, items: Array<GanttItem>): Array<number> {
  let ends: Array<number> = [0];
  let heights: Array<number> = [];
  for (let i in items) {
    let item = items[i];
    let start = dayjs(item.startTime, "%Y-%m-%d %H:%M:%S").diff(startTime, 'minute');
    let end = dayjs(item.endTime, "%Y-%m-%d %H:%M:%S").diff(startTime, 'minute');

    let flag = false;
    for (let j in ends) {
      if (start >= ends[j]) {
        heights.push(Number(j));
        ends[j] = end;
        flag = true;
        break;
      }
    }

    if (!flag) {
      heights.push(ends.length);
      ends.push(end);
    }
  }
  return heights;
}

export default defineComponent({
  name: "GanttChartLine",

  components: {
    GanttBar
  },

  setup() {
    const width = ref<number>(0);
    const height = ref<number>(baseLineHeight);
    const items = ref<Array<GanttItemView>>([]);
    const widthPerUnit = ref<number>(0);
    const linkedKey = ref<string>();

    const emit = defineEmits(['focused']);

    function init(
      startTime: dayjs.Dayjs,
      ganttItems: Array<GanttItem>,
      charWidth: number,
      chartWidthPerUnit: number,
      chartLinkedKey: string
    ) {
      width.value = charWidth;
      const heights = calculateHeights(startTime, ganttItems);
      height.value = .25 + (Math.max.apply(Math, heights) + 1) * baseLineHeight;
      widthPerUnit.value = chartWidthPerUnit;
      linkedKey.value = chartLinkedKey;

      items.value = ganttItems.map((item, i) => {
        const x = dayjs(item.startTime, "%Y-%m-%d %H:%M:%S").diff(startTime, 'minute');
        const y = .25 + heights[i] * baseLineHeight;
        const result: GanttItemView = {
          x: x,
          y: y,
          item: item,
          view: null
        };
        return result;
      });
    }

    function setViewRefs(el: HTMLElement | ComponentPublicInstance | HTMLAttributes, item: GanttItemView) {
      if (el) {
        const view = el as typeof GanttBar;
        item.view = view;
        item.view.init(item.item, widthPerUnit.value, linkedKey.value);
      }
    }

    function rescale(scale: number) {
      for (const item of items.value) {
        item.x = item.x * scale;
        if (item.view != null) {
          item.view.rescale(scale);
        }
      }
    }

    function focus(linkedKey: string, linkedInfo: string) {
      for (const item of items.value) {
        if (item.view != null) {
          item.view.focus(linkedKey, linkedInfo);
        }
      }
    }

    async function focused(linkedInfo: string) {
      emit("focused", linkedInfo)
    }

    return {
      width,
      height,
      items,
      init,
      setViewRefs,
      rescale,
      focus,
      focused
    };
  }
});
</script>
