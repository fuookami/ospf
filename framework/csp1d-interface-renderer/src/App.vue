<template>
  <v-app>
    <v-main :style="{ height: windowHeight + 'px' }" style="display:flex; flex-direction: column;">
      <FileSelector ref="fileSelector" @fileLoadingSucceeded="renderData" @fileLoadingFailed="showMessage"/>
      <v-container :style="{ 'visibility': rendererVisibility }" style="width: 100%; max-width: 100%; padding: 0; margin: 0;">
        <CuttingPlanGroupView v-for="group of groups" :ref="(el) => setViewRefs(el, group)" style="width: 100%;"/>
      </v-container>

      <v-dialog v-model="dialog" persistent max-width="30%">
        <v-card>
          <v-card-title class="headline">错误信息</v-card-title>
          <v-card-text v-html="message"></v-card-text>
          <v-card-actions>
            <v-spacer></v-spacer>
            <v-btn color="green darken-1" @click="dialog = false">关闭</v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>
    </v-main>
  </v-app>
</template>

<script lang="ts">
import {defineComponent, ComponentPublicInstance, HTMLAttributes, onMounted, ref} from "vue";

import {Schema} from "./components/dto.ts";
import FileSelector from "./components/file-selector.vue";
import CuttingPlanGroupView from "./components/cutting-plan-group-view.vue";

export default defineComponent({
  name: "App",

  components: {
    FileSelector,
    CuttingPlanGroupView
  },

  setup() {
    const windowHeight = ref(0);
    const windowWidth = ref(0);
    const rendererVisibility = ref("hidden");
    const message = ref("");
    const dialog = ref(false);

    const schema = ref<Schema>({
      cuttingPlans: [],
      kpi: new Map()
    })
    const groups = ref<Array<Array<string>>>([]);
    const cuttingPlanViews: Array<typeof CuttingPlanGroupView> = [];

    onMounted(() => {
      resized();
      window.addEventListener("resize", () => {
        resized();
      });
    })

    function isSameGroup(group1: Array<string>, group2: Array<string>): boolean {
      if(group1.length !== group2.length) {
        return false;
      }

      for(let i = 0; i < group1.length; i++) {
        if(group1[i] !== group2[i]) {
          return false;
        }
      }

      return true;
    }

    function distinctGroups(groups: Array<Array<string>>): Array<Array<string>> {
      const result: Array<Array<string>> = [];
      for(let i = 0; i < groups.length; i++) {
        if (groups[i].length === 0) {
          result.push(groups[i]);
        } else {
          let isDistinct = true;
          for(let j = 0; j < result.length; j++) {
            if(isSameGroup(groups[i], result[j])) {
              isDistinct = false;
              break;
            }
          }
          if(isDistinct) {
            result.push(groups[i]);
          }
        }
      }
      return result;
    }

    function setViewRefs(el: HTMLElement | ComponentPublicInstance | HTMLAttributes, group: Array<string>) {
      if(el) {
        const view = el as typeof CuttingPlanGroupView;
        view.windowHeight = windowHeight.value;
        view.windowWidth = windowWidth.value;
        view.group = group;
        view.cuttingPlans = schema.value.cuttingPlans.filter((p) => isSameGroup(p.group, group));
        cuttingPlanViews.push(view);
      }
    }

    function resized() {
      windowWidth.value = window.innerWidth;
      windowHeight.value = window.innerHeight;

      for (const view of cuttingPlanViews) {
        view.resized(windowHeight.value, windowWidth.value);
      }
    }

    function renderData(targetSchema: Schema) {
      rendererVisibility.value = "visible";
      schema.value = targetSchema;
      groups.value = distinctGroups(targetSchema.cuttingPlans.map((p) => p.group));
    }

    function showMessage(msg: string) {
      message.value = msg;
      dialog.value = true;
    }

    return {
      windowHeight,
      windowWidth,
      rendererVisibility,
      message,
      dialog,
      schema,
      groups,
      cuttingPlanViews,
      setViewRefs,
      renderData,
      showMessage
    }
  }
});
</script>
