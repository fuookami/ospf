<template>
  <v-container fluid style="margin: 0; padding: 0; display: flex; flex-direction: row;">
    <v-text-field style="flex: 1;" v-model="fileName" readonly clearable hide-details label="输入文件"/>
    <v-btn class="align-self-center" @click="selectFile">选择输入文件</v-btn>
  </v-container>
</template>

<script lang="ts">
import {defineComponent, ref} from "vue";
import {invoke} from '@tauri-apps/api/core';
import {open} from '@tauri-apps/plugin-dialog';

export default defineComponent({
  name: "FileSelector",

  setup(_, context) {
    const fileName = ref("");

    async function selectFile() {
      const selected = await open({
        title: "选择数据文件",
        directory: false,
        multiple: false,
        defaultPath: await invoke('get_current_directory',),
        filters: [{
          name: 'json',
          extensions: ['json']
        }]
      });
      if (selected === null) {
        context.emit("fileLoadingFailed", "未选择文件")
      } else {
        fileName.value = selected;
      }
      invoke("load_data", {path: fileName.value}).then(
          (data) => context.emit("fileLoadingSucceeded", data),
          (err) => context.emit("fileLoadingFailed", err)
      );
    }

    return {
      fileName,
      selectFile
    };
  }
});
</script>
