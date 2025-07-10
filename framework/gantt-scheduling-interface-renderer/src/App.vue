<template>
  <v-app>
    <v-main ref="main" :style="{ height: windowHeight + 'px' }" style="display:flex; flex-direction: column;">
      <FileSelector ref="fileSelector" @fileLoadingSucceeded="render" @fileLoadingFailed="showMessage" />
      <gantt-chart-render 
        ref="ganttChart" 
        :style="{ 'visibility': rendererVisibility }"
        style="width: 100%; flex: 1;" 
      />

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
import { onMounted, defineComponent, ref } from "vue";

import FileSelector from "./components/file-selector.vue";
import GanttChartRender from "./components/gantt-chart.vue";
import { Schema } from "./components/dto";

export default defineComponent({
  name: "App",

  components: {
    FileSelector,
    GanttChartRender
  },

  setup() {
    const main = ref<HTMLDivElement>();
    const fileSelector = ref<typeof FileSelector | null>();
    const ganttChart = ref<typeof GanttChartRender | null>();

    const windowHeight = ref(0);
    const windowWidth = ref(0);
    const rendererVisibility = ref("hidden");
    const message = ref("");
    const dialog = ref(false);

    onMounted(() => {
      resized();
      window.addEventListener("resize", () => {
        resized();
      });
    })

    function render(data: Schema) {
      if (ganttChart.value) {
        rendererVisibility.value = "visible";
        ganttChart.value!!.reder(data)
      }
    }

    function resized() {
      windowWidth.value = window.innerWidth;
      windowHeight.value = window.innerHeight;
      if (ganttChart.value) {
        ganttChart.value!!.resize(ganttChart.value!!.offsetWidth, main.value!!.offsetHeight - fileSelector.value!!.offsetHeight);
      }
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
      render,
      resized,
      showMessage
    }
  }
});
</script>
