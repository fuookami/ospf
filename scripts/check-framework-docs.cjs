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
    const lesson = entries.find(([slug, , , lesson]) => page === `${slug}/${lesson}.md`)
    if (lesson) {
      assert.ok(!/\b(?:mvn\s.*\btest|cargo\s.*\btest)\b/.test(text), `${file}: keep regression commands in verification`)
      for (const extension of ['kt', 'rs']) {
        assert.ok(text.includes(`/${lesson[0]}.${extension}`), `${file}: missing ${extension} application example`)
      }
    }
    for (const match of text.matchAll(/^<<<\s+(\S+)/gm)) {
      const source = path.resolve(path.dirname(file), match[1])
      assert.ok(fs.existsSync(source), `${file}: missing example include ${match[1]}`)
      const code = read(source)
      assert.ok(!/assertIs|valueOrFail|Delegating|#\[test\]|kotlin\.test|org\.junit/.test(code), `${source}: example must not depend on a test harness`)
    }
    if (page.includes('/') && !page.endsWith('business-scenario.md')) {
      assert.ok(text.includes('](./business-scenario)'), `${file}: missing second-lesson link`)
      assert.ok(text.includes('](../integration)'), `${file}: missing integration link`)
    }
    if (page.endsWith('business-scenario.md')) {
      const entry = entries.find(([slug]) => page.startsWith(`${slug}/`))
      assert.ok(text.includes(`](./${entry[3]})`), `${file}: missing first-lesson prerequisite`)
      assert.ok(text.includes('](../integration)'), `${file}: missing integration link`)
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
