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
        <p v-html='`真实形状：${selectedItemShape}`' />
        <p v-html='`渲染形状：${selectedItemRenderShape}`' />
        <p v-html='`算法形状：${selectedItemAlgorithmShape}`' />
        <p v-if="selectedItemUnsupportedReason" style="color: #D32F2F; font-weight: 600;">
          不支持：{{ selectedItemUnsupportedReason }}
        </p>
        <p v-html='`包装尺寸：${selectedItemSize}`' />
        <p v-if="selectedItemBoundingSize" v-html='`外接尺寸：${selectedItemBoundingSize}`' />
        <p v-if="selectedItemAxis" v-html='`轴向：${selectedItemAxis}`' />
        <p v-if="selectedItemRadius" v-html='`半径：${selectedItemRadius}`' />
        <p v-if="selectedItemDiameter" v-html='`直径：${selectedItemDiameter}`' />
        <p v-if="selectedItemActualVolume" v-html='`实际体积：${selectedItemActualVolume}`' />
        <p v-html='`装载位置：${selectedItemPosition}`' />
        <p v-html='`装载顺序：${selectedItemLoadingOrder}`' />
        <p v-html='`箱数：${selectedItemAmount}`' />
        <p v-html='`重量：${selectedItemWeight}kg`' />
      </v-card-text>
      <v-card-text v-for="info in selectedItemInfo" :key="info.key">
        <p v-html='`${info.key}：${info.value}`' />
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
        <v-table density="compact" style="table-layout: fixed;">
          <tbody>
            <tr v-for="summary in loadingSummary" :key="summary.key">
              <td style="width: 8em;">{{ summary.key }}</td>
              <td>{{ summary.value }}</td>
            </tr>
          </tbody>
        </v-table>
        <v-table density="compact" style="table-layout: fixed;">
          <thead>
            <tr>
              <th class="text-center">形状</th>
              <th class="text-center">数量</th>
              <th class="text-center">实际体积</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="summary in shapeSummary" :key="summary.shape">
              <td class="text-center">{{ summary.shape }}</td>
              <td class="text-center">{{ summary.amount }}</td>
              <td class="text-center">{{ summary.actualVolume }}</td>
            </tr>
          </tbody>
        </v-table>
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
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { LoadingPlanDTO, LoadingPlanItemDTO } from './dto.ts';
import { cylinderAxisLength, formatSize, resolveLoadingItemShape, ResolvedLoadingItemShape, shapeDisplayName } from './loading-item-shape.ts';

type ComponentElementRef = {
  $el: HTMLElement
}

type LoadingItemVO = {
  item: LoadingPlanItemDTO
  type: string
  color: string
  amount: number
  shape: ResolvedLoadingItemShape
}

type LoadingStepVO = {
  order: number
  amount: number
  name: string
}

type SummaryRowVO = {
  key: string
  value: string
}

type ShapeSummaryVO = {
  shape: string
  amount: number
  actualVolume: string
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
  const shape = resolveLoadingItemShape(item);
  const packageType = item.packageType ?? '';
  if (shape.renderShapeType === 'Cylinder') {
    return [
      packageType,
      shape.renderShapeType,
      shape.axis,
      (shape.radius ?? 0).toFixed(2),
      cylinderAxisLength(shape).toFixed(0),
      item.weight.toFixed(2)
    ].join('-');
  }
  return `${packageType}-${shape.renderShapeType}-${item.width.toFixed(0)}*${item.height.toFixed(0)}*${item.depth.toFixed(0)}-${item.weight.toFixed(2)}`;
}

function getLoadingSteps(items: Array<LoadingPlanItemDTO>): Array<LoadingStepVO> {
  if (items.length === 0) {
    return [];
  }

  const maxStep = lodash.maxBy(items, 'loadingOrder')!!.loadingOrder;

  const steps: Array<LoadingStepVO> = [];
  for (let i = 0; i <= maxStep; i++) {
    const names: Array<string> = [];
    for (const item of items) {
      if (item.loadingOrder == i) {
        names.push(item.name);
      }
    }
    steps.push({
      order: i,
      amount: names.length,
      name: Array.from(names.reduce((counter, currentValue) => {
        counter.set(currentValue, (counter.get(currentValue) ?? 0) + 1);
        return counter;
      }, new Map<string, number>()).entries()).map(([name, amount]) => {
        return `${name} * ${amount}`
      }).join(";")
    });
  }
  return steps;
}

function actualVolumeOf(item: LoadingPlanItemDTO, shape: ResolvedLoadingItemShape): number {
  return shape.actualVolume ?? item.width * item.height * item.depth;
}

function createLoadingSummary(loadingPlan: LoadingPlanDTO): Array<SummaryRowVO> {
  const actualVolume = loadingPlan.items.reduce((sum, item) => {
    return sum + actualVolumeOf(item, resolveLoadingItemShape(item));
  }, 0);
  const actualLoadingRate = loadingPlan.width * loadingPlan.height * loadingPlan.depth > 0
    ? actualVolume / (loadingPlan.width * loadingPlan.height * loadingPlan.depth)
    : 0;

  return [
    { key: '货柜', value: `${loadingPlan.group.join("-")}-${loadingPlan.name}` },
    { key: '类型', value: loadingPlan.typeCode },
    { key: '尺寸', value: formatSize(loadingPlan.width, loadingPlan.height, loadingPlan.depth) },
    { key: '物品数量', value: `${loadingPlan.items.length}` },
    { key: '重量', value: `${loadingPlan.weight.toFixed(2)}kg` },
    { key: 'DTO体积', value: loadingPlan.volume.toFixed(2) },
    { key: '实际体积', value: actualVolume.toFixed(2) },
    { key: 'DTO装载率', value: `${(loadingPlan.loadingRate * 100).toFixed(2)}%` },
    { key: '实际装载率', value: `${(actualLoadingRate * 100).toFixed(2)}%` }
  ];
}

function createShapeSummary(items: Array<LoadingPlanItemDTO>): Array<ShapeSummaryVO> {
  const summaries = new Map<string, { amount: number, actualVolume: number }>();
  for (const item of items) {
    const shape = resolveLoadingItemShape(item);
    const key = `${shape.shapeType}/${shape.renderShapeType}/${shape.algorithmShapeType}`;
    const summary = summaries.get(key) ?? { amount: 0, actualVolume: 0 };
    summary.amount += 1;
    summary.actualVolume += actualVolumeOf(item, shape);
    summaries.set(key, summary);
  }
  return Array.from(summaries.entries()).map(([shape, summary]) => {
    return {
      shape,
      amount: summary.amount,
      actualVolume: summary.actualVolume.toFixed(2)
    };
  });
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
    const shape = resolveLoadingItemShape(item);
    const vo: LoadingItemVO = {
      item: item,
      type: type,
      color: itemTypeColors.get(type)!!,
      amount: itemTypeAmount.get(type)!!,
      shape: shape
    }
    const mesh = createItemMesh(item, loadingPlan, vo);
    items.set(mesh, vo);
  }
  return items;
}

function createItemMesh(item: LoadingPlanItemDTO, loadingPlan: LoadingPlanDTO, vo: LoadingItemVO): THREE.Mesh {
  const color = getItemRenderColor(vo);
  const darkenColor = darken(color, 0.33);
  const geometry = createItemGeometry(vo.shape);
  const material = new THREE.MeshBasicMaterial({
    color: color,
    transparent: true,
    depthWrite: false
  });
  const mesh = new THREE.Mesh(geometry, material);
  mesh.userData.loadingPlanItemMesh = true;
  mesh.userData.loadingPlanItemName = item.name;
  applyCylinderAxisRotation(mesh, vo.shape);

  const edges = new THREE.EdgesGeometry(geometry);
  const line = new THREE.Line(edges, new THREE.LineBasicMaterial({
    color: darkenColor,
    linewidth: 1
  }));
  line.userData.loadingPlanItemMesh = mesh;
  mesh.add(line);
  mesh.position.set(
    item.x + vo.shape.boundingWidth / 2 - loadingPlan.width / 2,
    item.y + vo.shape.boundingHeight / 2 - loadingPlan.height / 2,
    item.z + vo.shape.boundingDepth / 2 - loadingPlan.depth / 2
  );
  return mesh;
}

function createItemGeometry(shape: ResolvedLoadingItemShape): THREE.BufferGeometry {
  if (shape.unsupportedReason) {
    console.error(shape.unsupportedReason);
    return new THREE.BoxGeometry(shape.boundingWidth, shape.boundingHeight, shape.boundingDepth);
  }

  if (shape.renderShapeType === 'Cylinder') {
    return new THREE.CylinderGeometry(shape.radius!!, shape.radius!!, cylinderAxisLength(shape), 48);
  }

  return new THREE.BoxGeometry(shape.boundingWidth, shape.boundingHeight, shape.boundingDepth);
}

function applyCylinderAxisRotation(mesh: THREE.Mesh, shape: ResolvedLoadingItemShape) {
  if (shape.renderShapeType !== 'Cylinder' || shape.unsupportedReason) {
    return;
  }

  if (shape.axis === 'X') {
    mesh.rotation.z = -Math.PI / 2;
  } else if (shape.axis === 'Z') {
    mesh.rotation.x = Math.PI / 2;
  }
}

function findLoadingItemMesh(object: THREE.Object3D): THREE.Mesh | null {
  let current: THREE.Object3D | null = object;
  while (current) {
    if (current instanceof THREE.Mesh && current.userData.loadingPlanItemMesh === true) {
      return current;
    }
    current = current.parent;
  }
  return null;
}

function setItemEdgesVisible(mesh: THREE.Mesh, visible: boolean) {
  for (const line of mesh.children) {
    if (line instanceof THREE.Line) {
      (line.material as THREE.LineBasicMaterial).visible = visible;
    }
  }
}

function setItemVisual(mesh: THREE.Mesh, color: THREE.Color, opacity: number) {
  const material = mesh.material as THREE.MeshBasicMaterial;
  material.opacity = opacity;
  material.color = color;
}

function getItemRenderColor(item: LoadingItemVO): THREE.Color {
  return item.shape.unsupportedReason ? new THREE.Color('#FF3366') : new THREE.Color(item.color);
}

function getItemInfoRows(item: LoadingPlanItemDTO): Array<{ key: string, value: string }> {
  return Object.entries(item.info ?? {}).map(([key, value]) => {
    return { key, value };
  });
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
    const rendererContainer = ref<ComponentElementRef | null>();
    const loadingStepTable = ref<ComponentElementRef | null>();

    const items = ref<Map<THREE.Mesh, LoadingItemVO>>();

    const selectedItemInfoVisibility = ref('hidden');
    const selectedItemName = ref('');
    const selectedItemPackageType = ref('');
    const selectedItemShape = ref('');
    const selectedItemRenderShape = ref('');
    const selectedItemAlgorithmShape = ref('');
    const selectedItemUnsupportedReason = ref('');
    const selectedItemSize = ref('');
    const selectedItemBoundingSize = ref('');
    const selectedItemAxis = ref('');
    const selectedItemRadius = ref('');
    const selectedItemDiameter = ref('');
    const selectedItemActualVolume = ref('');
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
    const loadingSummary = ref<Array<SummaryRowVO>>([]);
    const shapeSummary = ref<Array<ShapeSummaryVO>>([]);
    const loadingStepNameWidth = ref<string>('160px');

    function init(loadingPlan: LoadingPlanDTO) {
      rendererContainer.value!!.$el.innerHTML = '';

      const renderer = new THREE.WebGLRenderer();
      const scene = new THREE.Scene();
      scene.background = new THREE.Color('#e7e7e7');

      items.value = createItems(loadingPlan);
      loadingSteps.value = getLoadingSteps(loadingPlan.items);
      loadingSummary.value = createLoadingSummary(loadingPlan);
      shapeSummary.value = createShapeSummary(loadingPlan.items);
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

      rendererContainer.value!!.$el.addEventListener("resize", (_: Event) => {
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

      rendererContainer.value!!.$el.addEventListener("dblclick", (event: MouseEvent) => {
        event.preventDefault();
        const raycaster = new THREE.Raycaster();
        const mouse = new THREE.Vector2();
        mouse.x = (event.offsetX / rendererContainer.value!!.$el.offsetWidth) * 2 - 1;
        mouse.y = -(event.offsetY / rendererContainer.value!!.$el.offsetHeight) * 2 + 1;
        raycaster.setFromCamera(mouse, camera);
        const intersects = raycaster.intersectObjects(scene.children, true);
        const selectedObject = intersects.length != 0 ? findLoadingItemMesh(intersects[0].object) : null;
        if (selectedObject) {
          const selectedItem = items.value!!.get(selectedObject)!!;

          setItemVisual(selectedObject, lighten(getItemRenderColor(selectedItem), 0.2), 1.0);
          for (const obj of scene.children) {
            if (obj instanceof THREE.Mesh) {
              setItemEdgesVisible(obj, true);

              const item = items.value!!.get(obj);
              if (item) {
                if (item.type === selectedItem.type && item != selectedItem) {
                  setItemVisual(obj, getItemRenderColor(item), 0.9);
                } else if (item != selectedItem) {
                  setItemVisual(obj, getItemRenderColor(item), 0.3);
                }
              }
            }
          }
          selectedItemInfoVisibility.value = "visible";
          selectedItemName.value = selectedItem.item.name;
          selectedItemPackageType.value = selectedItem.item.packageType ?? '';
          selectedItemShape.value = selectedItem.shape.shapeType;
          selectedItemRenderShape.value = shapeDisplayName(selectedItem.shape);
          selectedItemAlgorithmShape.value = selectedItem.shape.algorithmShapeType;
          selectedItemUnsupportedReason.value = selectedItem.shape.unsupportedReason ?? '';
          selectedItemSize.value = formatSize(selectedItem.item.width, selectedItem.item.height, selectedItem.item.depth);
          selectedItemBoundingSize.value = selectedItem.shape.shapeType === 'Cylinder'
              || selectedItem.item.boundingWidth != null
              || selectedItem.item.boundingHeight != null
              || selectedItem.item.boundingDepth != null
            ? formatSize(
              selectedItem.shape.boundingWidth,
              selectedItem.shape.boundingHeight,
              selectedItem.shape.boundingDepth
            )
            : '';
          selectedItemAxis.value = selectedItem.shape.shapeType === 'Cylinder' ? selectedItem.shape.axis : '';
          selectedItemRadius.value = selectedItem.shape.radius != null ? selectedItem.shape.radius.toFixed(2) : '';
          selectedItemDiameter.value = selectedItem.shape.diameter != null ? selectedItem.shape.diameter.toFixed(2) : '';
          selectedItemActualVolume.value = selectedItem.shape.actualVolume != null ? selectedItem.shape.actualVolume.toFixed(2) : '';
          selectedItemPosition.value = `${selectedItem.item.x.toFixed(0)},${selectedItem.item.y.toFixed(0)},${selectedItem.item.z.toFixed(0)}`;
          selectedItemLoadingOrder.value = `${selectedItem.item.loadingOrder}`;
          selectedItemAmount.value = `${selectedItem.amount}`;
          selectedItemWeight.value = `${selectedItem.item.weight.toFixed(2)}`;
          selectedItemInfo.value = getItemInfoRows(selectedItem.item);
        } else {
          control.reset();
          for (const obj of scene.children) {
            if (obj instanceof THREE.Mesh) {
              setItemEdgesVisible(obj, true);
              const item = items.value!!.get(obj);
              if (item) {
                setItemVisual(obj, getItemRenderColor(item), 1.0);
              }
            }
          }

          selectedItemInfoVisibility.value = "hidden";
          selectedItemInfo.value = [];
          selectedItemUnsupportedReason.value = '';
          selectedItemRenderShape.value = '';
          selectedItemAlgorithmShape.value = '';
        }
      });

      loadingStepTable.value!!.$el.addEventListener("click", (event: MouseEvent) => {
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
                setItemEdgesVisible(obj, true);
                setItemVisual(obj, getItemRenderColor(item), item.item.loadingOrder == selectedOrder ? 1.0 : 0.3);
              } else {
                setItemEdgesVisible(obj, false);
                setItemVisual(obj, getItemRenderColor(item), 0.0);
              }
            }
          }
        }
      });

      animate();
    }

    watch(tab, (newTab, _) => {
      if (newTab != null) {
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
      selectedItemShape,
      selectedItemRenderShape,
      selectedItemAlgorithmShape,
      selectedItemUnsupportedReason,
      selectedItemSize,
      selectedItemBoundingSize,
      selectedItemAxis,
      selectedItemRadius,
      selectedItemDiameter,
      selectedItemActualVolume,
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
      loadingSummary,
      shapeSummary,
      loadingStepNameWidth,
      init
    }
  }
});
</script>
