<template>
  <div class="gantt_header" ref="header" :style="{ width: width + 'px' }">
    <div class="gantt_meta_line" ref="metaLine"
      :style="{ width: ganttWidth + 'px', 'margin-left': maxNameWidth + 'em' }"
    >
      <p v-for="(header, _) in metaHeaders" :style="{ 'min-width': header.width + 'px' }">
        {{ header.name }}
      </p>
    </div>

    <div class="gantt_meta_line" ref="metaAssistantLine"
      :style="{ width: ganttWidth + 'px', 'margin-left': maxNameWidth + 'em' }
    ">
      <p v-for="(header, _) in metaSubHeaders" :style="{ 'min-width': header.width + 'px' }">
        {{ header.name }}
      </p>
    </div>
  </div>

  <div :style="{ width: width + 'px' }" style="padding: 0; display: flex;">
    <div class="gantt_actor" ref="actor" :style="{ height: chartHeight + 'px' }" @mousewheel.prevent>
      <p
        v-for="(line, _) in lines" 
        :style="{
          width: maxNameWidth + 'em',
          'line-height': line.height,
          display: line.visible
        }"
      >
        {{ line.name }}
      </p >
    </div>

    <div
      ref="chart" 
      :style="{ width: chartWidth + 'px', height: chartHeight + 'px' }"
      style="overflow-x: auto; overflow-y: auto;" 
      @scroll="chartScroll()"
    >
      <div
        v-for="(line, _) in lines"
        style="
          min-height: 3.5em;
          padding: .25em 0 .25em 0;
          border-bottom: 1px dotted #5a5a5a;
        "
        :style="{
          width: ganttWidth + 'px',
          display: line.visible
        }"
      >
        <gantt-chart-line-view 
          :ref="(el) => setViewRefs(el, line)"
          @focused="focused"
        />
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import "./chart.css"
import {ComponentPublicInstance, defineComponent, defineExpose, defineEmits, HTMLAttributes, ref} from "vue";
import dayjs from "dayjs";
import GanttChartLineView from "./line.vue";
import {GanttChartVO, GanttLineVO} from "../vo.ts";

const minWidthPerHour = 32;
const scales = [1 / 12, 1 / 8, 1 / 6, 1 / 4, 1 / 3, 1 / 2, 1, 2, 4, 8, 16, 32, 64];

type GanttLineView = {
  name: string
  height: number
  visible: string
  line: GanttLineVO
  view: typeof GanttChartLineView | null
}

type GanttHeader = {
  name: string
  width: number
}

function generateHeader(
  startTime: dayjs.Dayjs, 
  endTime: dayjs.Dayjs, 
  widthPerHour: number, 
  stepHours: number
): Array<Array<GanttHeader>> {
  let diffHours = endTime.diff(startTime, "hour");
  const startDate = startTime.startOf("day");
  const endDate = endTime.startOf("day");
  let dayHours = startTime.hour();
  const metaSubHeaders: Array<GanttHeader> = [];
  const metaHeaders: Array<GanttHeader> = [];
  for (let date = startDate; date <= endDate; date = date.add(1, "day")) {
    let hours = Math.min(diffHours, 24 - dayHours);
    if (hours === 0) {
      break;
    }
    const width = widthPerHour * hours;

    if (hours % stepHours !== 0 && dayHours !== 0) {
      const thisStepHours = stepHours + hours % stepHours;
      if (hours === thisStepHours) {
        metaSubHeaders.push({
          width: width,
          name: `${dayHours % 24}`
        });
      } else {
        metaSubHeaders.push({
          width: widthPerHour * thisStepHours,
          name: `${dayHours % 24}`
        });
      }
      hours -= thisStepHours;
      diffHours -= thisStepHours;
      dayHours = (dayHours + thisStepHours) % 24;
    }

    for (let j = 0; j < hours;) {
      const thisStepHours = Math.min(hours - j, stepHours);
      metaSubHeaders.push({
        width: widthPerHour * thisStepHours,
        name: `${dayHours % 24}`
      });
      j += thisStepHours;
      dayHours = (dayHours + thisStepHours) % 24
    }
    metaHeaders.push({
      width: width,
      name: `${date.month() + 1}-${date.date()}`
    });
    dayHours = 0;
    diffHours -= hours;
  }
  return [metaSubHeaders, metaHeaders];
}

export default defineComponent({
  name: "GanttChartView",

  components: {
    GanttChartLineView
  },

  setup() {
    const chart = ref<HTMLElement>();
    const header = ref<HTMLElement>();
    const metaLine = ref<HTMLElement>();
    const metaAssistantLine = ref<HTMLElement>();
    const actor = ref<HTMLElement>();

    const width = ref(0);
    const chartWidth = ref(0);
    const chartHeight = ref(0);
    const ganttWidth = ref(0);
    const ganttHeight = ref(0);
    const diffHours = ref(0);

    const lines = ref<Array<GanttLineView>>([]);
    const maxNameWidth = ref(0);

    const metaHeaders = ref<Array<GanttHeader>>([]);
    const metaSubHeaders = ref<Array<GanttHeader>>([]);

    const widthPerHour = ref<number>(minWidthPerHour);
    const startTime = ref<dayjs.Dayjs>();
    const endTime = ref<dayjs.Dayjs>();
    const stepHours = ref<number>(1);

    const linkedKeys = ref<Array<string>>([]);
    const enabledLinkedKeys = ref<Array<string>>([]);

    const emit = defineEmits(['focused']);

    function init(schema: GanttChartVO, windowWidth: number, windowHeight: number) {
      schema.lines.sort((lhs, rhs) => {
        if (lhs.name < rhs.name) {
          return -1;
        } else if (lhs.name > rhs.name) {
          return 1;
        } else {
          return 0;
        }
      });
      startTime.value = dayjs(schema.startTime, "%Y-%m-%d %H:%M:%S").subtract(1, "hour").startOf("hour");
      endTime.value = dayjs(schema.endTime, "%Y-%m-%d %H:%M:%S").add(2, "hour").startOf("hour");
      diffHours.value = endTime.value.diff(startTime.value, "hour");
      widthPerHour.value = Math.ceil(Math.max(minWidthPerHour, windowWidth / diffHours.value));
      maxNameWidth.value = 0;
      const newLines: Array<GanttLineView> = [];
      for (const line of schema.lines) {
        maxNameWidth.value = Math.max(maxNameWidth.value, line.name.length);
        const newLine: GanttLineView = {
          name: line.name,
          height: 16,
          visible: "",
          line: line,
          view: null
        }
        newLines.push(newLine);
      }
      lines.value = newLines;

      const [subHeaders, headers] = generateHeader(startTime.value, endTime.value, widthPerHour.value, stepHours.value);
      metaSubHeaders.value = subHeaders;
      metaHeaders.value = headers;

      width.value = windowWidth;
      chartHeight.value = windowHeight - metaLine.value!!.offsetHeight - metaAssistantLine.value!!.offsetHeight;
      chartWidth.value = width.value - maxNameWidth.value * 16;
      ganttWidth.value = widthPerHour.value * diffHours.value;
      ganttHeight.value = 0;
      linkedKeys.value = schema.linkInfo;
      enabledLinkedKeys.value = schema.linkInfo;
    }

    function setViewRefs(el: HTMLElement | ComponentPublicInstance | HTMLAttributes, line: GanttLineView) {
      if (el) {
        line.view = el as typeof GanttChartLineView;
        line.view.init(startTime.value, line.line.items, chartWidth.value, widthPerHour.value / 60, linkedKeys.value);
        let newHeight = 0;
        for (const line of lines.value) {
          if (line.view) {
            line.height = line.view!!.height;
            newHeight += line.height * 16;
          }
        }
        ganttHeight.value = newHeight;
      }
    }

    function resize(newWidth: number, newHeight: number) {
      width.value = newWidth;
      chartHeight.value = newHeight - metaLine.value!!.offsetHeight - metaAssistantLine.value!!.offsetHeight;
      chartWidth.value = width.value - maxNameWidth.value * 16;

      if (startTime != null && endTime != null) {
        const newWidthPerHour = Math.ceil(Math.max(minWidthPerHour, width.value / diffHours.value));
        if (newWidthPerHour !== widthPerHour.value) {
          const scale = newWidthPerHour / widthPerHour.value;
          ganttWidth.value = ganttWidth.value * scale;
          widthPerHour.value = newWidthPerHour;
          for (const line of lines.value) {
            line.view!!.rescale(scale);
          }
          for (const header of metaHeaders.value) {
            header.width = header.width * scale;
          }
          for (const header of metaSubHeaders.value) {
            header.width = header.width * scale;
          }
        }
      }
    }

    function rescale(currentScale: number, newScale: number) {
      const scale = scales[newScale] / scales[currentScale];
      const newWidthPerHour = widthPerHour.value * scale;
      const newStepHours = Math.ceil(minWidthPerHour / newWidthPerHour);

      ganttWidth.value = ganttWidth.value * scale;
      widthPerHour.value = newWidthPerHour;
      for (const line of lines.value) {
        line.view!!.rescale(scale);
      }
      if (newStepHours === stepHours.value) {
        for (const header of metaHeaders.value) {
          header.width = header.width * scale;
        }
        for (const header of metaSubHeaders.value) {
          header.width = header.width * scale;
        }
      } else {
        const [newMetaSubHeaders, newMetaHeaders] = generateHeader(startTime.value!!, endTime.value!!, newWidthPerHour, newStepHours);
        metaHeaders.value = newMetaHeaders;
        metaSubHeaders.value = newMetaSubHeaders;
      }
    }

    function setVisibleLines(visibleLines: Array<string>) {
      for (const line of lines.value) {
        if (visibleLines.includes(line.name)) {
          console.log(line.name);
          line.visible = "";
        } else {
          line.visible = "none";
        }
      }
    }

    async function chartScroll() {
      const maxScrollX = ganttWidth.value - width.value + maxNameWidth.value * 16;
      const maxScrollY = ganttHeight.value;
      if (chart.value!!.scrollLeft > maxScrollX) {
        chart.value!!.scrollLeft = maxScrollX;
      }
      if (chart.value!!.scrollTop > maxScrollY) {
        chart.value!!.scrollTop = maxScrollY;
      }
      header.value!!.scrollLeft = chart.value!!.scrollLeft;
      actor.value!!.scrollTop = chart.value!!.scrollTop;
    }

    async function focused(linkedInfo: Map<string, string>) {
      emit("focused", linkedInfo);
      for (const line of lines.value) {
        line.view!!.focus(linkedInfo);
      }
    }

    defineExpose({
      scales
    });

    return {
      chart,
      header,
      metaLine,
      metaAssistantLine,
      actor,
      width,
      chartWidth,
      chartHeight,
      ganttWidth,
      ganttHeight,
      lines,
      maxNameWidth,
      metaHeaders,
      metaSubHeaders,
      scales,
      init,
      setViewRefs,
      resize,
      rescale,
      setVisibleLines,
      chartScroll,
      focused
    };
  }
});
</script>
../vo