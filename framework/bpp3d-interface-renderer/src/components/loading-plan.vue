<template>
  <div style="width: 100%; height: 100%; display:flex; flex-direction: column;">
    <div class="d-flex flex-row">
      <v-select style="max-width: 15em;" v-model="selectedBin" label="选择货柜" :items="bins" no-data-text="没有货"
        hide-details />
    </div>
    <bin-loading-plan ref="loadingPlanView" :stype="{ 'visibility': loadingPlanVisibility }" />
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, watch } from "vue";
import BinLoadingPlan from "./bin-loading-plan.vue";
import { SchemaDTO } from './dto.ts';

export default defineComponent({
  name: "LoadingPlan",

  components: {
    BinLoadingPlan
  },

  setup() {
    const loadingPlanView = ref<typeof BinLoadingPlan | null>();

    const schema = ref<SchemaDTO>();
    const bins = ref<Array<string>>([]);
    const selectedBin = ref<string | null>();
    const loadingPlanVisibility = ref<string>("hidden");

    function init(data: SchemaDTO) {
      schema.value = data;
      loadingPlanVisibility.value = "hidden";

      bins.value = data.loadingPlans.map(bin => `${bin.group.join("-")}-${bin.name}`);
      selectedBin.value = null;
    }

    watch(selectedBin, (newSelectedBin): void => {
      if (newSelectedBin) {
        const loadingPlan = schema.value!!.loadingPlans.find(bin => `${bin.group.join("-")}-${bin.name}` === newSelectedBin);
        if (loadingPlan) {
          loadingPlanVisibility.value = "visible";
          loadingPlanView.value!!.init(loadingPlan);
        } else {
          loadingPlanVisibility.value = "hidden";
        }
      } else {
        loadingPlanVisibility.value = "hidden";
      }
    });

    return {
      loadingPlanView,
      bins,
      selectedBin,
      loadingPlanVisibility,
      init
    };
  }
});
</script>
