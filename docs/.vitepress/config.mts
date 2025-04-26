import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "ospf",
  description: "ospf document",

  locales: {
    root: {
      label: 'English',
      lang: 'en-us',
      themeConfig: {
        nav: [
          { text: 'Home', link: '/' },
          { text: 'Guide', link: '/guide/what-is-ospf' }
        ],
        sidebar: {
          '/guide': [
            {
              text: 'Introduction',
              items: [
                { 'text': 'What is OSPF?', link: '/guide/what-is-ospf' },
                { 'text': 'Getting Started', link: '/guide/getting-started' },
              ]
            }
          ],
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ]
      }
    },
    'zh-cn': {
      label: '简体中文',
      lang: 'zh-cn',
      themeConfig: {
        nav: [
          { text: '主页', link: '/zh-cn/' },
          { text: '指南', link: '/zh-cn/guide/what-is-ospf' }
        ],
        sidebar: {
          '/zh-cn/guide': [
            {
              text: '简介',
              items: [
                { 'text': 'OSPF 是什么?', link: '/zh-cn/guide/what-is-ospf' },
                { 'text': '快速开始', link: '/zh-cn/guide/getting-started' },
              ]
            }
          ],
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ]
      }
    }
  }
})
