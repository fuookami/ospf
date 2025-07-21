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

function isSameGroup(group1: Array<String>, group2: Array<String>): boolean {
  if (group1.length !== group2.length) {
    return false;
  }

  for (let i = 0; i < group1.length; i++) {
    if (group1[i] !== group2[i]) {
      return false;
    }
  }

  return true;
}

function distinctGroups(groups: Array<Array<String>>): Array<Array<String>> {
  const result: Array<Array<String>> = [];
  for (let i = 0; i < groups.length; i++) {
    if (groups[i].length === 0) {
      result.push(groups[i]);
    } else {
      let isDistinct = true;
      for (let j = 0; j < result.length; j++) {
        if (isSameGroup(groups[i], result[j])) {
          isDistinct = false;
          break;
        }
      }
      if (isDistinct) {
        result.push(groups[i]);
      }
    }
  }
  return result;
}

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
