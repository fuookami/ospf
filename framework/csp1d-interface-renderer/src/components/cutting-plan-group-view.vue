<template>
  <h4 v-for="title in group">{{ title }}</h4>
  <CuttingPlanView v-for="cuttingPlan of cuttingPlans" :ref="(el) => setViewRefs(el, cuttingPlan)" style="width: 100%; max-width: 100%; height: 1.5em; "/>
</template>

<script lang="ts">
import {ComponentPublicInstance, defineComponent, HTMLAttributes, ref} from "vue";
import CuttingPlanView from "./cutting-plan-view.vue";
import {offset, CuttingPlan, CuttingPlanProduction} from "./dto.ts";

const defectCostarColor: number = 0x757575;
const costarColor: number = 0xEC4141;

const productColors = [
    0x01DDFF,
    0x5EF031,
    0xFF9600,
    0xFFDB01,
    0xF25A26,
    0xB768EB,
    0x18CB5F,
    0x854FFF,
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

const productMapper: string[] = [];

function getProductColor(category: string) {
    let index = productMapper.indexOf(category);
    if (index === -1) {
        index = productMapper.length;
        productMapper.push(category);
    }
    return productColors[index % productColors.length];
}

export default defineComponent({
  name: "CuttingPlanGroupView",

  components: {
    CuttingPlanView
  },

  setup() {
    const windowHeight = ref(0);
    const windowWidth = ref(0);

    const group = ref<Array<string>>([]);
    const cuttingPlans = ref<Array<CuttingPlan>>([]);
    const cuttingPlanViews: Array<typeof CuttingPlanView> = [];

    function setViewRefs(el: HTMLElement | ComponentPublicInstance | HTMLAttributes, cuttingPlan: CuttingPlan) {
      if(el) {
        const view = el as typeof CuttingPlanView;
        view.cuttingPlan = cuttingPlan;
        cuttingPlanViews.push(view);
        renderCuttingPlans();
      }
    }

    function resized(height: number, width: number) {
      windowWidth.value = width;
      windowHeight.value = height;

      for (let i = 0; i < cuttingPlanViews.length; i++) {
        let view = cuttingPlanViews[i];
        const scale = view.standardWidth / (windowWidth.value - offset);

        view.standardWidth = view.standardWidth / scale;
        view.productions.forEach((p: CuttingPlanProduction) => {
          p.x = (p.x - offset) / scale + offset;
          p.width = p.width / scale;
        });
      }
    }

    function renderCuttingPlans() {
      const standardWidth = Math.max.apply(null, cuttingPlans.value.map((x) => x.standardWidth));
      const scale = standardWidth / (windowWidth.value - offset);

      for (let i = 0; i < cuttingPlanViews.length; i++) {
        const view = cuttingPlanViews[i];
        view.standardWidth = view.cuttingPlan.standardWidth / scale;
        view.productions = view.cuttingPlan.productions.map((p: CuttingPlanProduction) => {
          const info = new Map(Object.entries(p.info));
          let color = 0x000000;
          if (p.productionType == "Costar") {
            color = costarColor;
          } else if (p.productionType == "DefectCostar") {
            color = defectCostarColor;
          } else {
            let category = p.width.toString();
            if (info.has("id")) {
              category += "-" + info.get("id");
            }
            if (info.has("length")) {
              category += "-" + info.get("length");
            }
            if (info.has("unitWeight")) {
              category += "-" + info.get("unitWeight");
            }
            color = getProductColor(category);
          }
          const result: CuttingPlanProduction = {
            name: p.name,
            x: p.x / scale + offset,
            width: p.width / scale,
            productionType: p.productionType,
            info: info,
            color: "#" + color.toString(16).padStart(6, '0'),
          };
          return result;
        });
      }
    }

    return {
      windowHeight,
      windowWidth,
      group,
      cuttingPlans,
      cuttingPlanViews,
      setViewRefs,
      resized,
      renderCuttingPlans
    }
  }
})
</script>
