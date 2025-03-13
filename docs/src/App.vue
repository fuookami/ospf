<template>
  <v-app>
    <v-app-bar app>
      <v-container>
        <v-row class="align-center">
          <v-col cols="auto">
            <h3 v-html="$t('home.title')"/>
          </v-col>
          <v-col class="d-flex justify-end">
            <v-select
                :items="languageItems"
                item-title="label"
                item-value="code"
                v-model="selectedLanguage"
                solo
                @update:modelValue="updateLanguage"
            ></v-select>
          </v-col>
        </v-row>
      </v-container>
    </v-app-bar>
    <v-main>
      <v-context>
        <h4 v-html="$t('index.title')"/>
        <p v-html="$t('index.description')"/>
      </v-context>
    </v-main>
  </v-app>
</template>

<script lang="ts">
import {defineComponent} from 'vue';

export default defineComponent({
  data() {
    return {
      languageItems: [
        { label: '简体中文', code: 'zhCn' },
        { label: 'English', code: 'en' }
      ],
      selectedLanguage: 'zhCn'
    };
  },

  created() {
    // 从本地存储读取用户语言偏好
    const savedLang = localStorage.getItem('userLanguage') || 'zhCn';
    this.selectedLanguage = savedLang;
    this.setLocale(savedLang);
  },

  methods: {
    setLocale(language: string) {
      this.$i18n.locale = language;
    },

    updateLanguage(language: string) {
      console.log(language);
      this.selectedLanguage = language;
      this.setLocale(language);
      // 持久化存储用户选择
      localStorage.setItem('userLanguage', language);
    }
  }
});
</script>
