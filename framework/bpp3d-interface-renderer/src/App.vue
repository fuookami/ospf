<template>
  <v-app>
    <v-main :style="{ height: windowHeight + 'px' }" style="display:flex; flex-direction: column;">
      <FileSelector ref="fileSelector" @fileLoadingSucceeded="renderData" @fileLoadingFailed="showMessage"/>

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

import FileSelector from "./components/file-selector.vue";

export default defineComponent({
  name: "App",

  components: {
    FileSelector
  },

  setup() {
    const windowHeight = ref(0);
    const windowWidth = ref(0);
    const rendererVisibility = ref("hidden");
    const message = ref("");
    const dialog = ref(false);

    const groups = ref<Array<Array<String>>>([]);

    onMounted(() => {
      resized();
      window.addEventListener("resize", () => {
        resized();
      });
    })

    function isSameGroup(group1: Array<String>, group2: Array<String>): boolean {
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

    function distinctGroups(groups: Array<Array<String>>): Array<Array<String>> {
      const result: Array<Array<String>> = [];
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

    function resized() {
      windowWidth.value = window.innerWidth;
      windowHeight.value = window.innerHeight;

    }

    function renderData() {
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
      groups,
      resized,
      renderData,
      showMessage
    }
  }
});
</script>
