const frameworks = [
  ['bpp1d', '一维装箱 BPP1D', '1D Packing · BPP1D', 'first-packing', '第一次一维装箱', 'Your First 1D Packing'],
  ['bpp2d', '二维装箱 BPP2D', '2D Packing · BPP2D', 'first-layout', '第一次二维排样', 'Your First 2D Layout'],
  ['bpp3d', '三维装箱 BPP3D', '3D Packing · BPP3D', 'first-loading', '第一次三维装箱', 'Your First 3D Loading'],
  ['csp1d', '一维分切 CSP1D', '1D Cutting · CSP1D', 'first-slitting', '第一次卷材分切', 'Your First Slitting Plan'],
  ['csp2d', '二维套裁 CSP2D', '2D Cutting · CSP2D', 'first-cutting', '第一次板材套裁', 'Your First Cutting Plan'],
  ['gantt', '时间与资源排程 Gantt', 'Time and Resources · Gantt', 'first-schedule', '第一次活动排程', 'Your First Activity Schedule'],
  ['network', '网络优化 Network', 'Network Optimization', 'first-flow', '第一次网络流优化', 'Your First Network Flow']
] as const

const nextLessons = {
  bpp1d: ['多箱型、库存与成本', 'Container Types, Stock and Cost'],
  bpp2d: ['多板型、旋转与间距', 'Sheet Types, Rotation and Spacing'],
  bpp3d: ['多箱型与目标配置', 'Bin Types and Objectives'],
  csp1d: ['需求口径与业务规则', 'Demand Units and Business Rules'],
  csp2d: ['刀缝、工艺与卷材', 'Kerf, Cutting Process and Coils'],
  gantt: ['停机日历与滚动重排', 'Calendars and Rolling Recovery'],
  network: ['共享资源、缺货与联合模型', 'Shared Resources, Shortage and Embedding']
} as const

export function frameworkSidebar(chinese = false) {
  const base = chinese ? '/zh-cn/frameworks' : '/frameworks'
  return [
    {
      text: chinese ? '从业务问题开始' : 'Start with Your Business Problem',
      items: [
        { text: chinese ? '选择业务框架' : 'Choose a Framework', link: `${base}/` },
        { text: chinese ? '环境与运行方式' : 'Environment and Running Examples', link: `${base}/setup` },
        { text: chinese ? '接入自己的业务' : 'Integrate Your Business', link: `${base}/integration` },
        { text: chinese ? '仓库验证参考（可选）' : 'Repository Verification (Optional)', link: `${base}/verification` }
      ]
    },
    ...frameworks.map(([slug, zhTitle, enTitle, lesson, zhLesson, enLesson]) => ({
      text: chinese ? zhTitle : enTitle,
      collapsed: true,
      items: [
        { text: chinese ? '适用问题与学习路线' : 'Scope and Learning Path', link: `${base}/${slug}/` },
        { text: chinese ? zhLesson : enLesson, link: `${base}/${slug}/${lesson}` },
        { text: nextLessons[slug][chinese ? 0 : 1], link: `${base}/${slug}/business-scenario` }
      ]
    }))
  ]
}
