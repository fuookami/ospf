<template>
  <v-container :stype="{
    width: standardWidth + offset + 'px',
    'max-width': standardWidth + offset + 'px'
  }" style=" margin: 0 0 5px 0; padding: 0; position: relative; border-right: 1px dotted #000;">
    <div style="width: 2em; position: absolute; text-align: center;">{{ cuttingPlan.amount }}</div>
    <div style="width: 3em; position: absolute; left: 2em; text-align: center;">{{ cuttingPlan.width }}</div>
    <v-container v-for="product of productions" :style="{
        left: product.x + 'px',
        width: product.width + 'px',
        'max-width': product.width + 'px'
      }" style="height: 1.5em; padding: 0 5px 0 0; position: absolute;"
    >
      <v-container :style="{
          width: product.width + 'px',
          'max-width': product.width + 'px',
          'background-color': product.color
        }" style="text-align: center; height: 1.5em; padding: 0; border-right: 1px dotted #000;"
      >
        <span>{{ product.name }}</span>
      </v-container>
    </v-container>
  </v-container>
</template>

<script lang="ts">
import {defineComponent, ref} from 'vue';
import {offset, CuttingPlanDTO, CuttingPlanProductionDTO} from "./dto.ts";

export default defineComponent({
  name: 'CuttingPlanView',

  setup() {
    const cuttingPlan = ref<CuttingPlanDTO>({
      group: [],
      amount: 0,
      width: 0,
      standardWidth: 0,
      productions: [],
      info: new Map()
    });
    const standardWidth = ref(0);
    const productions = ref<Array<CuttingPlanProductionDTO>>([]);

    return {
      offset,
      cuttingPlan,
      standardWidth,
      productions
    }
  }
});
</script>
