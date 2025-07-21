<template>
  <v-row style="height: 100%; width: 100%; position: relative;">
    <v-card 
      class="mx-auto" 
      :style="{ 'visibility': selectedItemInfoVisibility }"
      style="position: absolute; z-index: 1000; top: 1em; left: 1em; max-width: 25em;"
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
      </v-card-text>
      <v-card-text v-for="(key, value) in selectedItemInfo">
        <p v-html='`${key}：${value}`' />
      </v-card-text>
    </v-card>

    <v-col cols="9" ref="rendererContainer" height="100%" />
    <v-col ref="tabContainer" cols="2" height="100%">
      <v-tabs ref="tabList" v-model="tab" color="deep-purple-accent-4" align-tabs="center">
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
        <v-table ref="loadingStepTable" density="compact" style="table-layout: fixed;">
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
              <td 
                :style="{ 'width': loadingStepNameWidth }"
                style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
              >
                {{ step.name }}
              </td>
            </tr>
          </tbody>
        </v-table>
      </div>
    </v-col>
  </v-row>
</template>

<script lang="ts">
import {defineComponent, ref, toRaw, watch} from "vue";
import lodash from "lodash";
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls';
import { LoadingPlanDTO, LoadingPlanItemDTO } from './dto.ts';

type LoadingItemVO = {
  item: LoadingPlanItemDTO
  type: string
  color: string
  amount: number
}

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

function getItemType(item: LoadingPlanItemDTO): string {
  return `${item.packageType}-${item.width.toFixed(0)}*${item.height.toFixed(0)}*${item.depth.toFixed(0)}-${item.weight.toFixed(2)}`;
}

function getLoadingSteps(items: Array<LoadingPlanItemDTO>): Array<LoadingStepVO> {
  const maxStep = lodash.maxBy(items, 'loadingOrder')!!.loadingOrder;

  const steps: Array<LoadingStepVO> = [];
  for (let i = 0; i <= maxStep; i++) {
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

function createItems(loadingPlan: LoadingPlanDTO): Map<THREE.Mesh, LoadingItemVO> {
  const items = new Map<THREE.Mesh, LoadingItemVO>();
  const itemTypeColors = new Map<string, string>();
  const itemTypeAmount = new Map<string, number>();
  for (const item of loadingPlan.items) {
    const type = getItemType(item);
    if (!itemTypeColors.has(type)) {
      const color = itemColors[itemTypeColors.size % itemColors.length];
      itemTypeColors.set(type, color);
    }
    if (itemTypeAmount.has(type)) {
      itemTypeAmount.set(type, itemTypeAmount.get(type)!! + 1);
    } else {
      itemTypeAmount.set(type, 1);
    }
  }
  for (const item of loadingPlan.items) {
    const type = getItemType(item);
    const vo: LoadingItemVO = {
      item: item,
      type: type,
      color: itemTypeColors.get(type)!!,
      amount: itemTypeAmount.get(type)!!
    }
    const color = new THREE.Color(vo.color);
    const darkenColor = darken(color, 0.33);
    const geometry = new THREE.BoxGeometry(item.width, item.height, item.depth);
    const material = new THREE.MeshBasicMaterial({
      color: color,
      transparent: true,
      depthWrite: false
    });
    const cube = new THREE.Mesh(geometry, material);
    const edges = new THREE.EdgesGeometry(geometry);
    const line = new THREE.Line(edges, new THREE.LineBasicMaterial({
      color: darkenColor,
      linewidth: 1
    }));
    cube.add(line);
    cube.position.set(
      item.x + item.width / 2 - loadingPlan.width / 2,
      item.y + item.height / 2 - loadingPlan.height / 2,
      item.z + item.depth / 2 - loadingPlan.depth / 2
    );
    items.set(cube, vo);
  }
  return items;
}

function createBinLines(loadingPlan: LoadingPlanDTO): Array<THREE.Line> {
  const axesMaterial = new THREE.LineBasicMaterial({
    color: new THREE.Color('#3B65AC'),
    linewidth: 5
  });
  const scaleMaterial = new THREE.LineBasicMaterial({
    color: new THREE.Color('#3B65AC'),
    linewidth: 10
  });
  const lineLen = 200;
  const gap = 500;
  const scale = 1000;

  const lines = new Array<THREE.Line>();

  // x axes
  lines.push(new THREE.Line(new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(-loadingPlan.width / 2, -loadingPlan.height / 2, loadingPlan.depth / 2 + gap),
    new THREE.Vector3(loadingPlan.width / 2, -loadingPlan.height / 2, loadingPlan.depth / 2 + gap)
  ]), axesMaterial));

  for (let x = 0; ; x = Math.min(x + scale, loadingPlan.width)) {
    lines.push(new THREE.Line(new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(x - loadingPlan.width / 2, -loadingPlan.height / 2, loadingPlan.depth / 2 + gap - lineLen / 2),
      new THREE.Vector3(x - loadingPlan.width / 2, -loadingPlan.height / 2, loadingPlan.depth / 2 + gap + lineLen / 2)
    ]), scaleMaterial));

    if (x === loadingPlan.width) {
      break;
    }
  }

  // y axes
  lines.push(new THREE.Line(new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(loadingPlan.width / 2 + gap, -loadingPlan.height / 2, -loadingPlan.depth / 2),
    new THREE.Vector3(loadingPlan.width / 2 + gap, loadingPlan.height / 2, -loadingPlan.depth / 2)
  ]), axesMaterial));

  for (let y = 0; ; y = Math.min(y + scale, loadingPlan.height)) {
    lines.push(new THREE.Line(new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(loadingPlan.width / 2 + gap - lineLen / 2, y - loadingPlan.height / 2, -loadingPlan.depth / 2),
      new THREE.Vector3(loadingPlan.width / 2 + gap + lineLen / 2, y - loadingPlan.height / 2, -loadingPlan.depth / 2),
    ]), scaleMaterial));

    if (y === loadingPlan.height) {
      break;
    }
  }

  // z axes
  lines.push(new THREE.Line(new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(loadingPlan.width / 2 + gap, -loadingPlan.height / 2, -loadingPlan.depth / 2),
    new THREE.Vector3(loadingPlan.width / 2 + gap, -loadingPlan.height / 2, loadingPlan.depth / 2)
  ]), axesMaterial));

  for (let z = 0; ; z = Math.min(z + scale, loadingPlan.depth)) {
    lines.push(new THREE.Line(new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(loadingPlan.width / 2 + gap - lineLen / 2, -loadingPlan.height / 2, z - loadingPlan.depth / 2),
      new THREE.Vector3(loadingPlan.width / 2 + gap + lineLen / 2, -loadingPlan.height / 2, z - loadingPlan.depth / 2),
    ]), scaleMaterial));

    if (z === loadingPlan.depth) {
      break;
    }
  }

  return lines;
}

function createCamera(scene: THREE.Scene, window: HTMLElement, loadingPlan: LoadingPlanDTO): THREE.PerspectiveCamera {
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

    const items = ref<Map<THREE.Mesh, LoadingItemVO>>();

    const selectedItemInfoVisibility = ref('hidden');
    const selectedItemName = ref('');
    const selectedItemPackageType = ref('');
    const selectedItemSize = ref('');
    const selectedItemPosition = ref('');
    const selectedItemLoadingOrder = ref('');
    const selectedItemAmount = ref('');
    const selectedItemWeight = ref('');
    const selectedItemInfo = ref<Array<{ key: string, value: string }>>([]);

    const tab = ref<number | null>();
    const tabVisibility = ref<Array<string>>(['hidden', 'hidden']);
    const tabHeight = ref<string>('500px');
    const tabWidth = ref<string>('500px');
    const loadingSteps = ref<Array<LoadingStepVO>>([]);
    const loadingStepNameWidth = ref<string>('160px');

    function init(loadingPlan: LoadingPlanDTO) {
      rendererContainer.value!!.$el.innerHTML = '';

      const renderer = new THREE.WebGLRenderer();
      const scene = new THREE.Scene();
      scene.background = new THREE.Color('#e7e7e7');

      items.value = createItems(loadingPlan);
      loadingSteps.value = getLoadingSteps(loadingPlan.items);
      const binLines = createBinLines(loadingPlan);
      for (const [obj, _] of items.value) {
        scene.add(toRaw(obj));
      }
      for (const line of binLines) {
        scene.add(toRaw(line));
      }

      const light = new THREE.AmbientLight(new THREE.Color('#999999'))
      const directionalLight = new THREE.DirectionalLight(new THREE.Color('#ffffff'), 1.0)
      directionalLight.position.set(scene.position.x, scene.position.y, scene.position.z);
      scene.add(light)
      scene.add(directionalLight)

      renderer.setSize(rendererContainer.value!!.$el.offsetWidth, rendererContainer.value!!.$el.offsetHeight);
      const camera = createCamera(scene, rendererContainer.value!!.$el, loadingPlan);
      renderer.render(scene, toRaw(camera));
      rendererContainer.value!!.$el.append(renderer.domElement);

      rendererContainer.value!!.$el.addEventListener("resize", (_) => {
        renderer.setSize(rendererContainer.value!!.$el.offsetWidth, rendererContainer.value!!.$el.offsetHeight);
        camera.aspect = rendererContainer.value!!.$el.offsetWidth / rendererContainer.value!!.$el.offsetHeight;
        camera.updateProjectionMatrix();
      });

      const control = new OrbitControls(camera, renderer.domElement);
      control.saveState();
      function animate() {
        requestAnimationFrame(animate);
        renderer.render(scene, camera);
      }

      rendererContainer.value!!.$el.addEventListener("dblclick", (event) => {
        event.preventDefault();
        const raycaster = new THREE.Raycaster();
        const mouse = new THREE.Vector2();
        mouse.x = (event.offsetX / rendererContainer.value!!.$el.offsetWidth) * 2 - 1;
        mouse.y = -(event.offsetY / rendererContainer.value!!.$el.offsetHeight) * 2 + 1;
        raycaster.setFromCamera(mouse, camera);
        const intersects = raycaster.intersectObjects(scene.children);
        if (intersects.length != 0 && intersects[0].object instanceof THREE.Mesh) {
          const selectedObject = intersects[0].object as THREE.Mesh;
          const selectedItem = items.value!!.get(selectedObject)!!;

          (selectedObject.material as THREE.MeshBasicMaterial).opacity = 1.0;
          (selectedObject.material as THREE.MeshBasicMaterial).color = lighten(new THREE.Color(selectedItem.color), 0.2);
          for (const obj of scene.children) {
            if (obj instanceof THREE.Mesh) {
              for (const line of obj.children) {
                ((line as THREE.Line).material as THREE.LineBasicMaterial).visible = true;
              }

              const item = items.value!!.get(obj);
              if (item) {
                if (item.type === selectedItem.type && item != selectedItem) {
                  ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).opacity = 0.9;
                  ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).color = new THREE.Color(item.color);
                } else if (item != selectedItem) {
                  ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).opacity = 0.3;
                  ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).color = new THREE.Color(item.color);
                }
              }
            }
          }
          selectedItemInfoVisibility.value = "visible";
          selectedItemName.value = selectedItem.item.name;
          selectedItemPackageType.value = selectedItem.item.packageType;
          selectedItemSize.value = `${selectedItem.item.depth.toFixed(0)}*${selectedItem.item.width.toFixed(0)}*${selectedItem.item.height.toFixed(0)}`;
          selectedItemPosition.value = `${selectedItem.item.x.toFixed(0)},${selectedItem.item.y.toFixed(0)},${selectedItem.item.z.toFixed(0)}`;
          selectedItemLoadingOrder.value = `${selectedItem.item.loadingOrder}`;
          selectedItemAmount.value = `${selectedItem.amount}`;
          selectedItemWeight.value = `${selectedItem.item.weight.toFixed(2)}`;
        } else {
          control.reset();
          for (const obj of scene.children) {
            if (obj instanceof THREE.Mesh) {
              for (const line of obj.children) {
                ((line as THREE.Line).material as THREE.LineBasicMaterial).visible = true;
              }
              const item = items.value!!.get(obj);
              if (item) {
                ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).opacity = 1.0;
                ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).color = new THREE.Color(item.color);
              }
            }
          }

          selectedItemInfoVisibility.value = "hidden";
        }
      });

      loadingStepTable.value!!.$el.addEventListener("click", (event) => {
        let target = event.target!! as HTMLElement;
        if (target.nodeName == "TD") {
          target = target.parentNode as HTMLElement;
        }
        const selectedOrder = parseInt(target.getAttribute("order")!!, 10);
        for (const obj of scene.children) {
          if (obj instanceof THREE.Mesh) {
            const item = items.value!!.get(obj);
            if (item) {
              if (item.item.loadingOrder <= selectedOrder) {
                for (const line of obj.children) {
                  ((line as THREE.Line).material as THREE.LineBasicMaterial).visible = true;
                }
                ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).opacity = item.item.loadingOrder == selectedOrder ? 1.0 : 0.3;
                ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).color = new THREE.Color(item.color);
              } else {
                for (const line of obj.children) {
                  ((line as THREE.Line).material as THREE.LineBasicMaterial).visible = false;
                }
                ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).opacity = 0.0;
                ((obj as THREE.Mesh).material as THREE.MeshBasicMaterial).color = new THREE.Color(item.color);
              }
            }
          }
        }
      });

      animate();
    }

    watch(tab, (newTab, _) => {
      if (newTab) {
        tabVisibility.value[newTab] = 'visible';
        for (let i = 0; i < tabVisibility.value.length; i++) {
          if (i != newTab) {
            tabVisibility.value[i] = 'hidden';
          }
        }
      } else {
        for (let i = 0; i < tabVisibility.value.length; i++) {
          tabVisibility.value[i] = 'hidden';
        }
      }
    });

    return {
      rendererContainer,
      loadingStepTable,
      selectedItemInfoVisibility,
      selectedItemName,
      selectedItemPackageType,
      selectedItemSize,
      selectedItemPosition,
      selectedItemLoadingOrder,
      selectedItemAmount,
      selectedItemWeight,
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
