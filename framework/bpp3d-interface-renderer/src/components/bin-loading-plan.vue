<template>
  <v-row style="height: 100%; width: 100%; position: relative;">
    <v-card 
      class="mx-auto" 
      max-width="25em" 
      :style="{ 'visibility': selectedItemInfoVisibility }"
      style="position: absolute; z-index: 1000; top: 1em; left: 1em;"
    >
      <v-card-text>
        <p style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
          物料名称：{{ selectedItemName }}
          <v-tooltip activator="parent" location="top" max-width="500px">{{ selectedItemName }}</v-tooltip>
        </p>
        <p v-html='`包装类型：${selectedItemPackageType}`' />
        <p v-html='`包装尺寸：${selectedItemSize}`' />
        <p v-html='`装载位置：${selectedItemPosition}`' />
        <p v-html='`装载顺序：${selectedItemLoadingOrder}`' />
        <p v-html='`箱数：${selectedItemAmount}`' />
        <p v-html='`重量：${selectedItemWeight}kg`' />
        <p v-html='`所在仓库：${selectedItemWarehouse}`' />
      </v-card-text>
      <v-card-text v-for="(key, value) in selectedItemInfo">
        <p v-html='`${key}：${value}`' />
      </v-card-text>
    </v-card>

    <v-col cols="9" id="renderer" height="100%" />
    <v-col id="tabContainer" cols="2" height="100%">
      <v-tabs id="tabList" v-model="tab" color="deep-purple-accent-4" align-tabs="center">
        <v-tab :value="0">货物统计</v-tab>
        <v-tab :value="1">装柜步骤</v-tab>
      </v-tabs>

      <div 
        :style="{ 'visibility': tabVisibility[0], 'height': tabHeight, 'width': tabWidth }"
        style="position: absolute; overflow-y: auto;"
      >
      </div>

      <div 
        :style="{ 'visibility': tabVisibility[1], 'height': tabHeight, 'width': tabWidth }"
        style="position: absolute; overflow: auto;"
      >
        <v-table id="loadingStepTable" density="compact" style="table-layout: fixed;">
          <thead>
            <tr>
              <th class="text-center" style="width: 2em; padding: 0;">步骤</th>
              <th class="text-center" style="width: 2em; padding: 0;">数量</th>
              <th class="text-center" :style="{ 'width': loadingStepNameWidth }">汇总信息</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="step in loadingSteps" :order='`${step.order}`' style="background-color: '#00000050;">
              <td class="text-center" style="width: 2em;">{{ step.order }}</td>
              <td class="text-center" style="width: 2em;">{{ step.amount }}</td>
              <td :style="{ 'width': loadingStepNameWidth }"
                style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{{ step.name }}
              </td>
            </tr>
          </tbody>
        </v-table>
      </div>
    </v-col>
  </v-row>
</template>

<script lang="ts">
import { defineComponent, ref } from "vue";
import lodash from "lodash";
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls';
import { LoadingPlanDTO, LoadingPlanItemDTO } from './dto.ts';

type LoadingStepVO = {
  order: number
  amount: number
  name: string
}

const itemColors = [
  '#01DDFF',
  '#5EF031',
  '#FF9600',
  '#FFDB01',
  '#F25A26',
  '#B768EB',
  '#18CB5F',
  '#854FFF',
  '#EC4141',
  '#425FFF',
  '#12C4C7',
  '#B82BE0',
  '#33ECB7',
  '#F9D215',
  '#009BEC',
  '#7233FE',
  '#B99AFE',
  '#81E267',
  '#9AD2FF',
  '#C0EA00',
  '#FEE89A',
  '#4E62AB',
  '#9E0142',
  '#469EB4',
  '#D6404E',
  '#87CFA4',
  '#F57547',
  '#CBE99D',
  '#FDB96A',
  '#F5FBB1'
];

function getLoadingSteps(items: Array<LoadingPlanItemDTO>): Array<LoadingStepVO> {
  var maxStep = lodash.maxBy(items, 'loadingOrder')!!.loadingOrder;

  const steps: Array<LoadingStepVO> = [];
  for (var i = 0; i <= maxStep; i++) {
    const names = [];
    for (const item of items) {
      if (item.loadingOrder == i) {
        names.push(item.name);
      }
    }
    steps.push({
      order: i,
      amount: names.length,
      name: Object.entries(names.reduce((counter, currentValue) => {
        if (currentValue in counter) {
          counter.set(currentValue, counter.get(currentValue)!! + 1);
        } else {
          counter.set(currentValue, 1);
        }
        return counter;
      }, new Map<string, number>())).map(([name, amount]) => {
        return `${name} * ${amount}`
      }).join(";")
    });
  }
  return steps;
}

function createItems(itemsData = [], binData, items) {
}

function createCamera(scene: THREE.Scene, window: HTMLElement, loadingPlan: LoadingPlanDTO): THREE.Camera {
  const width = window.offsetWidth;
  const height = window.offsetHeight;

  const camera = new THREE.PerspectiveCamera(
    50,
    width / height,
    20,
    loadingPlan.depth * 10
  );
  camera.position.x = loadingPlan.width * 1.5;
  camera.position.y = loadingPlan.height * 1.5;
  camera.position.z = loadingPlan.depth * 1.25;
  camera.lookAt(scene.position);
  return camera;
}

function lighten(color: THREE.Color, offset: number): THREE.Color {
  const hsl = { h: 0, s: 0, l: 0 };
  color.getHSL(hsl);
  return color.clone().offsetHSL(0, 0, hsl.l * offset);
}

function darken(color: THREE.Color, offset: number): THREE.Color {
  const hsl = { h: 0, s: 0, l: 0 };
  color.getHSL(hsl);
  return color.clone().offsetHSL(0, 0, -hsl.l * offset);
}

export default defineComponent({
  name: "BinLoadingPlan",

  setup() {
    const rendererContainer = ref<HTMLElement | null>();
    const loadingStepTable = ref<HTMLElement | null>();

    const selectedItemInfoVisibility = ref('hidden');
    const selectedItemName = ref('');
    const selectedItemPackageType = ref('');
    const selectedItemSize = ref('');
    const selectedItemPosition = ref('');
    const selectedItemLoadingOrder = ref('');
    const selectedItemAmount = ref('');
    const selectedItemWeight = ref('');
    const selectedItemWarehouse = ref('');
    const selectedItemInfo = ref<Array<{ key: string, value: string }>>([]);

    const tab = ref<number | null>();
    const tabVisibility = ref<Array<string>>(['hidden', 'hidden']);
    const tabHeight = ref<string>('500px');
    const tabWidth = ref<string>('500px');
    const loadingSteps = ref<Array<LoadingStepVO>>([]);
    const loadingStepNameWidth = ref<string>('160px');

    function init(loadingPlan: LoadingPlanDTO) {
      rendererContainer.value!!.innerHTML = '';

      const renderer = new THREE.WebGLRenderer();
      const scene = new THREE.Scene();
      scene.background = new THREE.Color('#e7e7e7');

      // add items

      const light = new THREE.AmbientLight(new THREE.Color('#999999'))
      const directionalLight = new THREE.DirectionalLight(new THREE.Color('#ffffff'), 1.0)
      directionalLight.position.set(scene.position.x, scene.position.y, scene.position.z);
      scene.add(light)
      scene.add(directionalLight)

      renderer.setSize(rendererContainer.value!!.offsetWidth, rendererContainer.value!!.offsetHeight);
      const camera = createCamera(scene, rendererContainer.value!!, loadingPlan);
      renderer.render(scene, camera);
      rendererContainer.value!!.append(renderer.domElement);

      rendererContainer.value!!.addEventListener("resize", (event) => {

      });

      const control = new OrbitControls(camera, renderer.domElement);
      control.saveState();
      function animate() {
        requestAnimationFrame(animate);
        renderer.render(scene, camera);
      };

      rendererContainer.value!!.addEventListener("dblclick", (event) => {

      });

      loadingStepTable.value!!.addEventListener("click", (event) => {
      });
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
      tab,
      tabVisibility,
      tabHeight,
      tabWidth,
      loadingSteps,
      loadingStepNameWidth,
      init
    }
  }
});
</script>
