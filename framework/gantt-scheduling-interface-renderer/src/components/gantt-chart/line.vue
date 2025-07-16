<template>
  <v-container 
    :style="{ width: width + 'px', height: height + 'em' }"
    style=" margin: 0; padding: 0; position: relative;
  ">
    <gantt-chart-item-view v-for="(item, _) of items" 
      :ref="(el) => setViewRefs(el, item)"
      :style="{ left: item.x + 'px', top: item.y + 'em' }" 
      style="position: absolute;"
      @focused="focused"
    />
  </v-container>
</template>

<script lang="ts">
import "./chart.css"
import {ComponentPublicInstance, defineComponent, defineExpose, defineEmits, HTMLAttributes, ref} from "vue";
import dayjs from "dayjs";
import GanttChartItemView from "./item.vue";
import {GanttItemVO} from "../vo.ts";

const baseLineHeight = 3.5;

type GanttItemView = {
  x: number
  y: number
  item: GanttItemVO
  view: typeof GanttChartItemView | null
}

function calculateHeights(startTime: dayjs.Dayjs, items: Array<GanttItemVO>): Array<number> {
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
  name: "GanttChartLineView",

  components: {
    GanttChartItemView
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
      ganttItems: Array<GanttItemVO>,
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
        const x = dayjs(item.startTime, "%Y-%m-%d %H:%M:%S").diff(startTime, 'minute') * chartWidthPerUnit;
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
        item.view = el as typeof GanttChartItemView;
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

    function focus(linkedInfo: Map<string, string>) {
      for (const item of items.value) {
        if (item.view != null) {
          item.view.focus(linkedInfo);
        }
      }
    }

    async function focused(linkedInfo: Map<string, string>) {
      emit("focused", linkedInfo)
    }

    defineExpose({
      height
    });

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
../vo.ts