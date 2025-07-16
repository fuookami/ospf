<template>
  <div style="width: 100%; height: 100%; display:flex; flex-direction: column;">
    <p class="transition-swing text-body-1" v-html="renderMessage" />
    <div class="d-flex flex-row">
      <v-select style="max-width: 15em;" v-model="selectedBin" label="选择货柜" :items="bins" no-data-text="没有货" hide-details />
      <p class="transition-swing text-body-2 align-self-end" v-html="selectedBinMessage" />
    </div>

    <bin-loading-plan ref="loadingPlanView" :stype="{ 'visibility': loadingPlanVisibility }"/>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import BinLoadingPlan from "./bin-loading-plan.vue";
import { SchemaDTO } from './dto.ts';

export default defineComponent({
  name: "LoadingPlan",

  components: {
    BinLoadingPlan
  },

  setup() {
    const loadingPlanView = ref<typeof BinLoadingPlan | null>();

    const bins = ref<Array<string>>([]);
    const selectedBin = ref<string | null>();
    const loadingPlanVisibility = ref<string>("hidden");

    function init(schema: SchemaDTO) {
      loadingPlanVisibility.value = "hidden";

      bins.value = schema.loadingPlan.map(bin => `${bin.group.join("-")}-${bin.name}`);
      selectedBin.value = null;
    }

    return {
      bins,
      selectedBin,
      loadingPlanVisibility,
      init
    };
  }
});
</script>
