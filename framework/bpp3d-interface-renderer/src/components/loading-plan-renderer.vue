<template>
  <v-container ref="renderer" fluid style="margin: 0; padding: 0; display: flex; flex-direction: row;" />

  <v-card class="mx-auto" max-width="25em" :style="{ 'visibility': selectedItemInfoVisibility }"
          style="position: absolute; z-index: 1000; top: 1em; left: 1em; ">
    <v-card-text>
      <p style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
        Name：{{ selectedItemName }}
        <v-tooltip activator="parent" location="top" max-width="500px">{{ selectedItemName }}</v-tooltip>
      </p>
      <p v-html='`Package：${selectedItemPackageType}`'/>
      <p v-html='`Size：${selectedItemSize}`'/>
      <p v-html='`Position：${selectedItemPosition}`'/>
      <p v-html='`Order：${selectedItemLoadingOrder}`'/>
      <p v-html='`Amount：${selectedItemAmount}`'/>
      <p v-html='`Weight：${selectedItemWeight}kg`'/>
      <p v-html='`Warehouse：${selectedItemWarehouse}`'/>
    </v-card-text>
    <v-card-text v-for="(key, value) in selectedItemInfo">
      <p v-html='`${key}：${value}`'/>
    </v-card-text>
  </v-card>
</template>

<script lang="ts">
import {defineComponent, ref} from "vue";
import * as Three from "three";
import {LoadingPlan} from "./dto.ts";

function lighten(color: Three.Color, offset: number): Three.Color {
  const hsl = {h: 0, s: 0, l: 0};
  color.getHSL(hsl);
  return color.clone().offsetHSL(0, 0, hsl.l * offset);
}

function darken(color: Three.Color, offset: number): Three.Color {
  const hsl = {h: 0, s: 0, l: 0};
  color.getHSL(hsl);
  return color.clone().offsetHSL(0, 0, -hsl.l * offset);
}

export default defineComponent({
  name: "FileSelector",

  setup(_, context) {
    const selectedItemInfoVisibility = ref(false);
    const selectedItemName = ref("");
    const selectedItemPackageType = ref("");
    const selectedItemSize = ref("");
    const selectedItemPosition = ref("");
    const selectedItemLoadingOrder = ref("");
    const selectedItemAmount = ref("");
    const selectedItemWeight = ref("");
    const selectedItemWarehouse = ref("");
    const selectedItemInfo = ref(new Map<string, string>());

    async function render(plan: LoadingPlan) {

    }

    return {
      selectedItemInfoVisibility,
      selectedItemName,
      selectedItemPackageType,
      selectedItemSize,
      selectedItemPosition,
      selectedItemLoadingOrder,
      selectedItemAmount,
      selectedItemWeight,
      selectedItemWarehouse,
      selectedItemInfo,
      render
    };
  }
});
</script>
