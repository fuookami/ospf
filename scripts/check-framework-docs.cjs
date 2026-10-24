const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const docs = path.join(root, 'docs')
const read = file => fs.readFileSync(file, 'utf8')
const registry = read(path.join(docs, '.vitepress/frameworks.ts'))
const entries = registry.split('\n')
  .filter(line => /^\s*\['/.test(line))
  .map(line => [...line.matchAll(/'([^']*)'/g)].map(match => match[1]))
assert.equal(entries.length, 7, 'Expected seven registered business frameworks')
const nextLessons = Object.fromEntries([...registry.matchAll(/^\s*(\w+): \['([^']*)', '([^']*)'\]/gm)]
  .map(([, slug, zh, en]) => [slug, [zh, en]]))
const pages = ['index.md', 'setup.md', 'integration.md', 'verification.md', ...entries.flatMap(([slug, , , lesson]) => [
  `${slug}/index.md`, `${slug}/${lesson}.md`, `${slug}/business-scenario.md`
])]
const commands = new Set()
let sourceLinks = 0
const jvmTarget = read(path.join(root, 'ospf-kotlin/pom.xml'))
  .match(/<kotlin\.compiler\.jvmTarget>(\d+)<\/kotlin\.compiler\.jvmTarget>/)?.[1]
assert.ok(jvmTarget, 'Missing Kotlin JVM target')

function filesUnder(directory, suffix) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const file = path.join(directory, entry.name)
    return entry.isDirectory()
      ? (['target', 'node_modules', '.git'].includes(entry.name) ? [] : filesUnder(file, suffix))
      : (entry.name.endsWith(suffix) ? [file] : [])
  })
}

function proseOnly(text) {
  // API names, source paths and executable examples retain their original spelling.
  return text.replace(/^```[^\n]*\n[\s\S]*?^```[^\n]*$/gm, '')
    .replace(/^<<<.*$/gm, '').replace(/`+[^`\n]*`+/g, '')
    .replace(/\]\([^)]+\)/g, ']')
}

function checkLink(file, href) {
  const github = href.match(/^https:\/\/github\.com\/fuookami\/(ospf-kotlin|ospf-rust)\/(?:blob|tree)\/main\/(.+?)(?:#.*)?$/)
  if (github) {
    const target = path.resolve(root, github[1], decodeURIComponent(github[2]))
    assert.ok(fs.existsSync(target), `${file}: missing source ${href}`)
    sourceLinks++
    return
  }
  if (/^(?:[a-z]+:|#)/i.test(href)) return
  const route = decodeURIComponent(href.split(/[?#]/)[0])
  if (!route) return
  const target = route.startsWith('/')
    ? path.join(docs, route.slice(1))
    : path.resolve(path.dirname(file), route)
  assert.ok([`${target}.md`, path.join(target, 'index.md'), target]
    .some(candidate => fs.existsSync(candidate) && fs.statSync(candidate).isFile()),
  `${file}: broken local link ${href}`)
}

for (const locale of ['frameworks', 'zh-cn/frameworks']) {
  const directory = path.join(docs, locale)
  const actual = filesUnder(directory, '.md').map(file => path.relative(directory, file).replaceAll('\\', '/')).sort()
  assert.deepEqual(actual, [...pages].sort(), `${locale}: locale/registry page mismatch`)
  for (const page of pages) {
    const file = path.join(directory, page)
    const text = read(file)
    assert.ok(!/下面没有测试 solver 或预设解|不是测试代码|contain no test harness|手算对照不是原生执行日志|不要用换目标来替代后端接线|编排和提取路径在该夹具下工作/.test(text),
      `${file}: use developer-facing guidance instead of authoring instructions`)
    const wording = proseOnly(text)
    if (locale.startsWith('zh-cn')) {
      assert.ok(!/接线|接缝|终态|口径|契约|兑现|夹具|成本|实箱|箱子|守卫|\b(?:incumbent|seed|RMP|final|trace|helper|adapter|mock|fake|oracle|smoke|schema|DTO)\b/.test(wording),
        `${file}: use consistent domain terms and explain technical names in Chinese`)
      if (page.startsWith('bpp1d/')) assert.ok(!/箱型|货槽/.test(wording), `${file}: BPP1D uses 容器 and 容器类型`)
      if (page.startsWith('csp1d/')) assert.ok(!/物料|切片|图案/.test(wording), `${file}: CSP1D uses 原料, 切条 and 切割方案`)
    } else {
      assert.ok(!/\b(?:fixture|oracle|seam)\b/i.test(wording), `${file}: use reader-facing descriptions`)
    }
    if (page === 'index.md') assert.ok(text.includes('{#terminology}'), `${file}: missing terminology reference`)
    if (page.endsWith('/index.md')) {
      const [, zhTitle, , lesson, zhLesson, enLesson] = entries.find(([slug]) => page === `${slug}/index.md`)
      const next = text.split(/## (?:继续实践|Continue Practicing)/)[1]
      assert.ok(next, `${file}: missing continuation section`)
      assert.equal(next.match(/\]\(\.\/([^)]+)\)/)?.[1], lesson,
        `${file}: next lesson must start with ${lesson}, not skip to the business scenario`)
      assert.ok(next.includes(locale.startsWith('zh-cn') ? zhLesson : enLesson), `${file}: missing first-lesson title`)
      assert.ok(text.includes('](../#terminology)'), `${file}: missing terminology link`)
      if (locale.startsWith('zh-cn')) {
        const title = zhTitle.replace(/ (\S+)$/, '（$1）')
        assert.equal(text.split(/\r?\n/)[0], `# ${title}：适用问题与学习路线`, `${file}: use a framework-specific overview heading`)
      }
    }
    const lesson = entries.find(([slug, , , lesson]) => page === `${slug}/${lesson}.md`)
    if (lesson) {
      assert.ok(!/\b(?:mvn\s.*\btest|cargo\s.*\btest)\b/.test(text), `${file}: keep regression commands in verification`)
      for (const extension of ['kt', 'rs']) {
        assert.ok(text.includes(`/${lesson[0]}.${extension}`), `${file}: missing ${extension} application example`)
        const source = read(path.join(docs, 'frameworks', '_examples', `${lesson[0]}.${extension}`))
        if (lesson[0] !== 'bpp1d') {
          assert.ok(source.includes('// #region result\n') || source.includes('// #region result\r\n'),
            `${file}: first lesson must expand actual result fields`)
        }
      }
    }
    for (const match of text.matchAll(/^<<<\s+(\S+)/gm)) {
      const [includePath, region] = match[1].split('#')
      const source = path.resolve(path.dirname(file), includePath)
      assert.ok(fs.existsSync(source), `${file}: missing example include ${match[1]}`)
      const code = read(source).replaceAll('\r\n', '\n')
      if (region) {
        assert.ok(code.includes(`// #region ${region}\n`) && code.includes(`// #endregion ${region}\n`),
          `${file}: missing example region ${region}`)
      }
      assert.ok(!/assertIs|valueOrFail|Delegating|#\[test\]|kotlin\.test|org\.junit/.test(code), `${source}: example must not depend on a test harness`)
    }
    if (page.includes('/') && !page.endsWith('business-scenario.md')) {
      assert.ok(text.includes('](./business-scenario)'), `${file}: missing second-lesson link`)
      assert.ok(text.includes('](../integration)'), `${file}: missing integration link`)
    }
    if (page.endsWith('business-scenario.md')) {
      const entry = entries.find(([slug]) => page.startsWith(`${slug}/`))
      const title = nextLessons[entry[0]][locale.startsWith('zh-cn') ? 0 : 1]
      assert.ok(text.startsWith(`# ${title}`), `${file}: business title must match the sidebar`)
      assert.ok(text.includes(`](./${entry[3]})`), `${file}: missing first-lesson prerequisite`)
      assert.ok(text.includes('](../integration)'), `${file}: missing integration link`)
      if (entry[0] === 'bpp1d') {
        for (const region of ['csv', 'policy', 'policy-injection', 'extension', 'extension-injection', 'cg']) {
          assert.ok(text.includes(`.rs#${region} `), `${file}: missing Rust ${region} example`)
          if (region !== 'policy-injection') assert.ok(text.includes(`.kt#${region} `), `${file}: missing Kotlin ${region} example`)
        }
      } else {
        for (const region of ['input', 'rule', 'entry']) {
          for (const extension of ['kt', 'rs']) {
            assert.ok(text.includes(`/${entry[0]}-business.${extension}#${region} `),
              `${file}: missing ${extension} ${region} example`)
          }
        }
        assert.ok(text.includes('{#entry-budget}'), `${file}: missing stable entry/budget anchor`)
        for (const region of entry[0] === 'csp2d' ? ['injection', 'master', 'cut'] : entry[0] === 'network' ? ['slack'] : []) {
          for (const extension of ['kt', 'rs']) {
            assert.ok(text.includes(`.${extension}#${region} `), `${file}: missing ${extension} ${region} wiring`)
          }
        }
      }
    }
    if (page === 'setup.md') {
      assert.ok(text.includes(`JDK ${jvmTarget}`), `${file}: update prerequisites for JVM ${jvmTarget}`)
    }
    assert.equal((text.match(/^```/gm) || []).length % 2, 0, `${file}: unclosed code fence`)
    const prose = text.replace(/^```[^\n]*\n[\s\S]*?^```\s*$/gm, '')
    assert.equal((prose.match(/^# /gm) || []).length, 1, `${file}: expected one page title`)
    for (const match of prose.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)) checkLink(file, match[1])
    for (const fence of text.matchAll(/^```powershell[^\n]*\n([\s\S]*?)^```/gm)) {
      const lines = fence[1].replace(/`\r?\n\s*/g, ' ').split(/\r?\n/)
      for (const line of lines) {
        if (/^(?:mvn|cargo)\s/.test(line)) {
          if (/\btest\b/.test(line)) {
            assert.equal(page, 'verification.md', `${file}: repository test commands belong in verification`)
          }
          commands.add(line.trim())
        }
      }
    }
  }
}

// Check source targets, not execution results: native availability is machine-specific.
for (const command of commands) {
  if (command.startsWith('mvn ')) {
    if (/^mvn\s+-version$/.test(command)) continue
    const moduleName = command.match(/-pl\s+(\S+)/)?.[1]
    assert.ok(moduleName, `Missing Kotlin module: ${command}`)
    const moduleRoot = path.join(root, 'ospf-kotlin', moduleName)
    const pom = read(path.join(moduleRoot, 'pom.xml'))
    const selection = command.match(/-Dtest=([^"\s]+)/)?.[1]
    assert.ok(selection, `Missing test selector: ${command}`)
    const [className, method] = selection.split('#')
    const file = filesUnder(path.join(moduleRoot, 'src'), '.kt')
      .find(candidate => path.basename(candidate) === `${className}.kt`)
    assert.ok(file, `Missing Kotlin test class ${selection}`)
    if (method) assert.ok(read(file).includes(`fun ${method}(`), `Missing Kotlin test ${selection}`)
    const profile = command.match(/-P([^\s"]+)/)?.[1]
    if (profile) assert.ok(pom.includes(`<id>${profile}</id>`), `Missing Maven profile ${profile}`)
  } else if (command.startsWith('cargo ')) {
    if (!command.includes(' test ')) continue
    assert.ok(command.startsWith('cargo +nightly-2026-08-01 test '), `Use the verified Rust toolchain: ${command}`)
    const crate = command.match(/-p\s+(\S+)/)?.[1]
    assert.ok(crate, `Missing Rust package: ${command}`)
    const crateRoot = path.join(root, 'ospf-rust', crate)
    const manifest = read(path.join(crateRoot, 'Cargo.toml'))
    const features = command.match(/--features\s+"([^"]+)"/)?.[1].split(',') || []
    for (const feature of features) {
      assert.ok(new RegExp(`^${feature}\\s*=`, 'm').test(manifest), `Missing ${crate} feature ${feature}`)
    }
    const target = command.match(/--test\s+(\S+)/)?.[1]
    const filter = target
      ? command.match(/--test\s+\S+\s+([\w]+)/)?.[1]
      : command.match(/--lib\s+([\w]+)/)?.[1]
    const sources = target
      ? [path.join(crateRoot, 'tests', `${target}.rs`)]
      : filesUnder(path.join(crateRoot, 'src'), '.rs')
    for (const source of sources) assert.ok(fs.existsSync(source), `Missing Rust test target ${source}`)
    if (filter) assert.ok(sources.some(source => read(source).includes(`fn ${filter}(`)), `Missing Rust test ${filter}`)
  }
}

console.log(`Framework docs: ${pages.length * 2} locale pages, ${sourceLinks} source links and ${commands.size} unique commands checked.`)
console.log('Static checks only; compile/run tests and inspect native skips separately.')
