<template>
  <v-container style="height: 3em; min-width: 0; padding: 0;">
    <v-btn 
      class="gantt_item" 
      :class="{
        'normal_item': isNormal,
        'testing_item': isTesting,
        'unavailable_item': isUnavailable,
        'unknown_item': isUnknown,
        'elevation-0': !isFocused,
        'evelation-1': isFocused,
        'focused_item': isFocused
      }"
      style="min-width: 0; padding: 0;" 
      :style="{
        width: width + 'px',
        'max-width': width + 'px',
        height: mainHeight + 'em',
        'margin-bottom': mainMarginBottom + 'em'
      }" 
      @click="click()"
    >
      <span :style="{ width: width + 'px' }">
        {{ name }}
      </span>
      <v-tooltip activator="parent" location="top">
        <p>Produce</p>
        <v-row v-for="(info, _) in produceList" style="margin: .25em 0 .25em 0; width: 20em;">
          <v-col cols="4" style="padding: 0;">{{ info.key }}</v-col>
          <v-col cols="1" style="padding: 0;">：</v-col>
          <v-col cols="7" style="padding: 0;">{{ info.value }}</v-col>
        </v-row>

        <p>Consumption</p>
        <v-row v-for="(info, _) in consumptionList" style="margin: .25em 0 .25em 0; width: 20em;">
          <v-col cols="4" style="padding: 0;">{{ info.key }}</v-col>
          <v-col cols="1" style="padding: 0;">：</v-col>
          <v-col cols="7" style="padding: 0;">{{ info.value }}</v-col>
        </v-row>

        <p>Info</p>
        <v-row v-for="(info, _) in infoList" style="margin: .25em 0 .25em 0; width: 20em;">
          <v-col cols="4" style="padding: 0;">{{ info.key }}</v-col>
          <v-col cols="1" style="padding: 0;">：</v-col>
          <v-col cols="7" style="padding: 0;">{{ info.value }}</v-col>
        </v-row>
      </v-tooltip>
      <v-snackbar v-model="snackbar" :timeout="2000">{{ snackbarText }}</v-snackbar>
    </v-btn>
    <v-container
      style="margin: 0; padding: 0; position: relative; min-width: 0;"
      :style="{
        width: width + 'px',
        height: subHeight + 'em'
      }" 
    >
      <div 
        v-for="(subItem, _) in subItems"
        class="gantt_item"
        style="position: absolute; height: 100%; position: absolute; text-align: center;"
        :style="{
          left: subItem.x + 'px',
          width: subItem.width + 'px',
          'max-width': subItem.width + 'px',
          background: subItem.color
        }"
      >
        <span 
          style="position: relative; top: -.75em; font-size: .5em;"
          :style="{ width: width + 'px' }" 
        >
          {{ subItem.name }}
        </span>
        <v-tooltip activator="parent" location="top">{{ subItem.name }}</v-tooltip>
      </div>
    </v-container>
  </v-container>
</template>

<script lang="ts">
import "./item.css"
import { defineComponent, defineEmits, ref } from "vue";
import clipoard3 from "vue-clipboard3";
import dayjs from "dayjs";
import { GanttItemVO, GanttSubItemVO } from "../vo.ts";

const subItemColors: number[] = [
  0x01DDFF,
  0x5EF031,
  0xFF9600,
  0xFFDB01,
  0xF25A26,
  0xB768EB,
  0x18CB5F,
  0x854FFF,
  0xEC4141,
  0x425FFF,
  0x12C4C7,
  0xB82BE0,
  0x33ECB7,
  0xF9D215,
  0x009BEC,
  0x7233FE,
  0xB99AFE,
  0x81E267,
  0x9AD2FF,
  0xC0EA00,
  0xFEE89A,
  0x4E62AB,
  0x9E0142,
  0x469EB4,
  0xD6404E,
  0x87CFA4,
  0xF57547,
  0xCBE99D,
  0xFDB96A,
  0xF5FBB1
];

const subItemMapper: string[] = [];

type GanttSubItemView = {
  name: string
  x: number
  width: number
  color: number
}

function getSubItemColor(category: string): number {
  let index = subItemMapper.indexOf(category);
  if (index === -1) {
    index = subItemMapper.length;
    subItemMapper.push(category);
  }
  return subItemColors[index % subItemColors.length];
}

function needSubBar(subItems: Array<GanttSubItemVO>): boolean {
  if (subItems.length == 0) {
    return false;
  } else {
    for (const subItem of subItems) {
      if (subItem.startTime !== subItem.endTime) {
        return true;
      }
    }
    return false;
  }
}

export default defineComponent({
  name: "GanttChartItemView",

  setup() {
    const item = ref<GanttItemVO>();
    const subItems = ref<Array<GanttSubItemView>>([]);

    const name = ref<string>("");
    const produceList = ref<Array<{ key: string, value: string }>>([]);
    const consumptionList = ref<Array<{ key: string, value: string }>>([]);
    const infoList = ref<Array<{ key: string, value: string }>>([]);
    const linkedInfo = ref<Map<string, string>>(new Map());

    const x = ref<number>(0);
    const y = ref<number>(0);
    const width = ref<number>(0);
    const mainHeight = ref<number>(3);
    const mainMarginBottom = ref<number>(0);
    const subHeight = ref<number>(0);

    const isNormal = ref<boolean>(false);
    const isTesting = ref<boolean>(false);
    const isUnavailable = ref<boolean>(false);
    const isUnknown = ref<boolean>(false);
    const isFocused = ref<boolean>(false);

    const snackbar = ref<boolean>(false);
    const snackbarText = ref<string>("");

    const emit = defineEmits(['focused']);
    const { toClipboard } = clipoard3();

    function init(item: GanttItemVO, widthPerUnit: number, linkedKeys: Array<string>) {
      name.value = item.name;
      infoList.value = Array.from(item.info).map(([key, value]) => ({ key, value }));
      if (linkedKeys != null && linkedKeys.length > 0) {
        const newLinkedInfo = new Map<string, string>();
        for (const key of linkedKeys) {
          const value = item.info.get(key);
          if (value != null) {
            newLinkedInfo.set(key, value);
          } else {
            newLinkedInfo.set(key, "");
          }
        }
        linkedInfo.value = newLinkedInfo;
      } else {
        linkedInfo.value = new Map();
      }
      const startTime = dayjs(item.startTime, "%Y-%m-%d %H:%M:%S");
      const endTime = dayjs(item.endTime, "%Y-%m-%d %H:%M:%S");
      const duration = endTime.diff(startTime, 'minute');
      width.value = duration * widthPerUnit;

      if (item.category === "Normal") {
        isNormal.value = true;
      } else if (item.category === "Testing") {
        isTesting.value = true;
      } else if (item.category === "Unavailable") {
        isUnavailable.value = true;
      } else if (item.category === "Unknown") {
        isUnknown.value = true;
      }

      if (needSubBar(item.subItems)) {
        mainHeight.value = 2;
        mainMarginBottom.value = .25;
        subHeight.value = .75;

        const subItemViews: Array<GanttSubItemView> = [];
        for (const subItem of item.subItems) {
          if (subItem.startTime === subItem.endTime) {
            continue;
          }

          const startTime = dayjs(item.startTime, "%Y-%m-%d %H:%M:%S");
          const subStartTime = dayjs(subItem.startTime, "%Y-%m-%d %H:%M:%S");
          const subEndTime = dayjs(subItem.endTime, "%Y-%m-%d %H:%M:%S");
          const subDuration = subEndTime.diff(subStartTime, 'minute');
          subItemViews.push({
            name: subItem.name,
            x: (subStartTime.diff(startTime, 'minute') * widthPerUnit) / duration,
            width: subDuration * widthPerUnit / duration,
            color: getSubItemColor(subItem.category)
          })
        }
        subItems.value = subItemViews;
      }
    }

    function rescale(scale: number) {
      x.value = x.value * scale;
      width.value = width.value * scale;
      for (const subItem of subItems.value) {
        subItem.x = subItem.x * scale;
        subItem.width = subItem.width * scale;
      }
    }

    function focus(linkedInfo: Map<string, string>) {
      let newIsFocused = false;
      for (const [linkedKey, linkedValue] of linkedInfo.entries()) {
        if (infoList.value.some(info => info.key === linkedKey && info.value === linkedValue)) {
          newIsFocused = true;
          break;
        }
      }
      isFocused.value = newIsFocused;
    }

    async function click() {
      try {
        await toClipboard(name.value);
        snackbarText.value = `${name} Copy Successfully`;
      } catch (e) {
        snackbarText.value = `${name} Copy Failed`;
      }
      snackbar.value = true;
      if (linkedInfo.value != null) {
        emit("focused", linkedInfo.value);
      }
    }

    return {
      item,
      subItems,
      name,
      produceList,
      consumptionList,
      infoList,
      linkedInfo,
      x,
      y,
      width,
      mainHeight,
      mainMarginBottom,
      subHeight,
      isNormal,
      isTesting,
      isUnavailable,
      isUnknown,
      isFocused,
      snackbar,
      snackbarText,
      init,
      rescale,
      focus,
      click
    };
  }
});
</script>
../vo.ts