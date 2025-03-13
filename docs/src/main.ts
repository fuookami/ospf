import { createApp } from 'vue'
import App from './App.vue'
import i18n from './i18n/index';
import vuetify from './plugins/vuetify'
import { loadFonts } from './plugins/webfontloader'

loadFonts()

createApp(App)
    .use(i18n)
    .use(vuetify)
    .mount('#app')
