import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "ospf",
  description: "ospf document",

  markdown: {
    math: true,
    lineNumbers: true
  },

  locales: {
    root: {
      label: 'English',
      lang: 'en-us',
      themeConfig: {
        nav: [
          { text: 'Home', link: '/' },
          { text: 'Guide', link: '/guide/what-is-ospf' },
          { text: 'Examples', link: '/examples/example1' }
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
          '/examples': [
            {
              text: 'Simple Examples',
              items: [
                { 'text': 'Example 1: Assigning problem', link: '/examples/example1' },
                { 'text': 'Example 2: Assigning problem', link: '/examples/example2' }
              ]
            }
          ]
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ],
        outline: {
          level: [2, 3]
        }
      }
    },
    'zh-cn': {
      label: '简体中文',
      lang: 'zh-cn',
      themeConfig: {
        nav: [
          { text: '主页', link: '/zh-cn/' },
          { text: '指南', link: '/zh-cn/guide/what-is-ospf' },
          { text: '示例', link: '/zh-cn/examples/example1' }
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
          '/zh-cn/examples': [
            {
              text: '简单示例',
              items: [
                { 'text': '示例 1：指派问题', link: '/zh-cn/examples/example1' },
                { 'text': '示例 2：指派问题', link: '/zh-cn/examples/example2' }
              ]
            }
          ]
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ],
        outline: {
          level: [2, 3]
        }
      }
    }
  }
})
