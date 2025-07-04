<template>
  <v-container fluid style="margin: 0; padding: 0;">
    <v-container ref="toolbar" fluid class="d-flex flex-row-reverse" style="margin: 0; padding: 0;">
      <v-btn density="compact" icon="mdi-plus" style="margin-right: .5em;" @click="rescale(1)" />
      <v-btn density="compact" icon="mdi-minus" style="margin-right: .5em;" @click="rescale(-1)" />
      <v-select v-model="visibleLines" :items="lines" label="可见行" multiple="true">
        <template v-slot:prepend-item>
          <v-list-item title="选择全部" @click="setAllLineVisible" />
          <v-list-item title="选择关联行" @click="setLinkedLineVisible" />
        </template>
      </v-select>
    </v-container>
    <gantt-chart ref="ganttChart" :style="{ height: height + 'px' }" @focus="focus" />
  </v-container>
</template>

<script lang="ts">
import {defineComponent, ref} from "vue";
import GanttChart from "./gantt-chart/chart.vue"
import { Schema } from "./dto.ts";

export default defineComponent({
  name: "GanttChart",

  components: {
    GanttChart
  },

  setup() {
    const schema = ref<Schema>();
    const height = ref(0);
    const minScale = ref(0);
    const maxScale = ref(0);
    const scale = ref(0);
    const lines = ref<Array<String>>([]);
    const visibleLines = ref<Array<String>>([]);
    const linkedLines = ref<Array<String>>([]);

    return {
      schema,
      height,
      minScale,
      maxScale,
      scale,
      lines,
      visibleLines,
      linkedLines
    }
  }
});

</script>
