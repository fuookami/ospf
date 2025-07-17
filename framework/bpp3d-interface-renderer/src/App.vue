<template>
  <v-app>
    <v-main :style="{ height: windowHeight + 'px' }" style="display:flex; flex-direction: column;">
      <file-selector ref="fileSelector" @fileLoadingSucceeded="renderData" @fileLoadingFailed="showMessage" />
      <loading-plan ref="loadingPlan" :style="{ visibility: rendererVisibility }" />

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
import { defineComponent, onMounted, ref } from "vue";
import FileSelector from "./components/file-selector.vue";
import LoadingPlan from "./components/loading-plan.vue";
import { SchemaDTO } from "./components/dto.ts";

export default defineComponent({
  name: "App",

  components: {
    FileSelector,
    LoadingPlan
  },

  setup() {
    const loadingPlan = ref<typeof LoadingPlan | null>();

    const windowHeight = ref(0);
    const windowWidth = ref(0);
    const rendererVisibility = ref("hidden");
    const message = ref("");
    const dialog = ref(false);

    const groups = ref<Array<Array<String>>>([]);

    function resized() {
      windowWidth.value = window.innerWidth;
      windowHeight.value = window.innerHeight;
    }

    function renderData(schema: SchemaDTO) {
      loadingPlan.value!!.init(schema);
      rendererVisibility.value = "visible";
    }

    function showMessage(msg: string) {
      message.value = msg;
      dialog.value = true;
    }

    onMounted(() => {
      resized();
      window.addEventListener("resize", () => {
        resized();
      });
    });

    return {
      loadingPlan,
      windowHeight,
      windowWidth,
      rendererVisibility,
      message,
      dialog,
      groups,
      resized,
      renderData,
      showMessage
    }
  }
});
</script>
