import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

// https://vitepress.dev/reference/site-config
export default withMermaid({
  title: "ospf",
  description: "ospf document",

  markdown: {
    math: true,
    lineNumbers: true,
    theme: {
      dark: "material-theme-darker",
      light: "material-theme-lighter"
    }
  },

  mermaid: {
    // Mermaid配置选项
  },

  mermaidPlugin: {
    class: 'mermaid'
  },

  locales: {
    root: {
      label: 'English',
      lang: 'en-us',
      themeConfig: {
        search: {
          provider: "local"
        },
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
                { 'text': 'Use Domain Driven Design Architecture', link: '/guide/use-ddd-architecture' }
              ]
            },
            {
              text: 'Advanced Applications',
              items: [
                { 'text': 'Linear Functional Intermediate Expression', link: '/guide/linear-functional-intermediate-expression' },
                { 'text': 'Quadratic Functional Intermediate Expression', link: '/guide/quadratic-functional-intermediate-expression' },
                { 'text': 'The Deductive Logic Expression of the Mathematical Model', link: '/guide/deductive-logic-expression' },
                { 'text': 'Formal Design and Formal Verification', link: '/guide/formal-design-and-formal-verification' },
                { 'text': 'Remote Solver', link: '/guide/remote-solver' },
                { 'text': 'Time Slice Cycle Solver', link: '/guide/time-slice-cycle-solver' }
              ]
            }
          ],
          '/examples': [
            {
              text: 'Simple Examples',
              items: [
                { 'text': 'Example 1: Assigning Problem', link: '/examples/example1' },
                { 'text': 'Example 2: Assigning Problem', link: '/examples/example2' },
                { 'text': 'Example 3: Ingredient Problem', link: '/examples/example3' },
                { 'text': 'Example 4: Ingredient Problem', link: '/examples/example4' },
                { 'text': 'Example 5: Knapsack Problem', link: '/examples/example5' },
                { 'text': 'Example 6: Knapsack Problem', link: '/examples/example6' },
                { 'text': 'Example 7: Transport Problem', link: '/examples/example7' },
                { 'text': 'Example 8: Production Problem', link: '/examples/example8' },
                { 'text': 'Example 9: Location Selection Problem', link: '/examples/example9' },
                { 'text': 'Example 10: Traveling Salesman Problem', link: '/examples/example10' },
                { 'text': 'Example 11: Max Flow Problem', link: '/examples/example11' },
                { 'text': 'Example 12: Portfolio Optimization Problem', link: '/examples/example12' },
                { 'text': 'Example 13: Two-Echelon Transport Problem', link: '/examples/example13' },
                { 'text': 'Example 14: Transshipment Problem', link: '/examples/example14' },
                { 'text': 'Example 15: Supply Transport Problem', link: '/examples/example15' },
                { 'text': 'Example 16: Production Inventory Problem', link: '/examples/example16' },
                { 'text': 'Example 17: Vehicle Routing Problem', link: '/examples/example17' },
              ]
            },
            {
              text: 'Complex Examples (with DDD Architecture)',
              items: [
                { 'text': 'Framework Example 1: Service Placement Problem', link: '/examples/framework-example1' },
                { 'text': 'Framework Example 2: Aircraft Cargo Load Planning Problem (with benders decomposition algorithm)', link: '/examples/framework-example2' },
                { 'text': 'Framework Example 3: Cutting Stock Problem 1D (with column generation algorithm)', link: '/examples/framework-example3' },
                { 'text': 'Framework Example 4: Flight Recovery Problem (with column generation algorithm)', link: '/examples/framework-example4' }
              ]
            }
          ]
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ],
        outline: {
          level: [2, 5]
        }
      }
    },
    'zh-cn': {
      label: '简体中文',
      lang: 'zh-cn',
      themeConfig: {
        search: {
          provider: "local"
        },
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
                { 'text': '使用领域驱动设计架构', link: '/zh-cn/guide/use-ddd-architecture' }
              ]
            },
            {
              text: '高级应用',
              items: [
                { 'text': '线性函数中间值', link: '/zh-cn/guide/linear-functional-intermediate-expression' },
                { 'text': '二次型函数中间值', link: '/zh-cn/guide/quadratic-functional-intermediate-expression' },
                { 'text': '数学模型的演绎逻辑表达', link: '/zh-cn/guide/deductive-logic-expression' },
                { 'text': '形式化设计与形式化验证', link: '/zh-cn/guide/formal-design-and-formal-verification' },
                { 'text': '云端求解器', link: '/zh-cn/guide/remote-solver' },
                { 'text': '时间片轮转求解器', link: '/zh-cn/guide/time-slice-cycle-solver' }
              ]
            }
          ],
          '/zh-cn/examples': [
            {
              text: '简单示例',
              items: [
                { 'text': '示例 1：指派问题', link: '/zh-cn/examples/example1' },
                { 'text': '示例 2：指派问题', link: '/zh-cn/examples/example2' },
                { 'text': '示例 3：配料问题', link: '/zh-cn/examples/example3' },
                { 'text': '示例 4：配料问题', link: '/zh-cn/examples/example4' },
                { 'text': '示例 5：背包问题', link: '/zh-cn/examples/example5' },
                { 'text': '示例 6：背包问题', link: '/zh-cn/examples/example6' },
                { 'text': '示例 7：运输问题', link: '/zh-cn/examples/example7' },
                { 'text': '示例 8：生产问题', link: '/zh-cn/examples/example8' },
                { 'text': '示例 9：选址问题', link: '/zh-cn/examples/example9' },
                { 'text': '示例 10：旅行商问题', link: '/zh-cn/examples/example10' },
                { 'text': '示例 11：最大流问题', link: '/zh-cn/examples/example11' },
                { 'text': '示例 12：投资组合问题', link: '/zh-cn/examples/example12' },
                { 'text': '示例 13：二级运输问题', link: '/zh-cn/examples/example13' },
                { 'text': '示例 14：转运问题', link: '/zh-cn/examples/example14' },
                { 'text': '示例 15：供给运输问题', link: '/zh-cn/examples/example15' },
                { 'text': '示例 16：生产库存问题', link: '/zh-cn/examples/example16' },
                { 'text': '示例 17：车辆路径问题', link: '/zh-cn/examples/example17' },
              ]
            },
            {
              text: '复杂示例（使用领域驱动设计架构）',
              items: [
                { 'text': '复杂示例 1：服务器放置问题', link: '/zh-cn/examples/framework-example1' },
                { 'text': '复杂示例 2：航空货运装载规划问题（使用 Benders 分解算法）', link: '/zh-cn/examples/framework-example2' },
                { 'text': '复杂示例 3：一维分切问题（使用列生成算法）', link: '/zh-cn/examples/framework-example3' },
                { 'text': '复杂示例 4：航班恢复问题（使用列生成算法）', link: '/zh-cn/examples/framework-example4' }
              ]
            }
          ]
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ],
        outline: {
          level: [2, 5]
        }
      }
    }
  }
})
