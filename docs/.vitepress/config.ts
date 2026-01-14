import { defineConfig } from 'vitepress'
import { readdirSync } from 'node:fs'
import { resolve } from 'node:path'

// 自动扫描 release-notes 目录
function getReleaseNotes(locale: 'en' | 'zh' = 'en') {
  const basePath = locale === 'zh' ? '../zh/release-notes' : '../release-notes'
  const linkPrefix = locale === 'zh' ? '/zh/release-notes' : '/release-notes'
  const dir = resolve(__dirname, basePath)
  const files = readdirSync(dir)
    .filter((f) => f.endsWith('.md'))
    .map((f) => f.replace('.md', ''))
    .sort((a, b) => {
      // 按版本号降序排列
      const [aMajor, aMinor, aPatch] = a.replace('v', '').split('.').map(Number)
      const [bMajor, bMinor, bPatch] = b.replace('v', '').split('.').map(Number)
      if (bMajor !== aMajor) return bMajor - aMajor
      if (bMinor !== aMinor) return bMinor - aMinor
      return bPatch - aPatch
    })

  return files.map((v) => ({ text: v, link: `${linkPrefix}/${v}` }))
}

const releaseNotes = getReleaseNotes('en')
const releaseNotesZh = getReleaseNotes('zh')

export default defineConfig({
  title: 'gcop-rs',
  description: 'AI-powered Git commit message generator',
  lastUpdated: true,
  ignoreDeadLinks: true,

  locales: {
    root: {
      label: 'English',
      lang: 'en',
    },
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      themeConfig: {
        lastUpdated: {
          text: '最后更新于',
        },
        nav: [
          { text: '指南', link: '/zh/guide/installation' },
          { text: '发布说明', link: releaseNotesZh[0]?.link || '/zh/release-notes/' },
          { text: '关于', link: '/zh/guide/about' },
        ],
        sidebar: {
          '/zh/guide/': [
            {
              text: '入门',
              items: [
                { text: '安装', link: '/zh/guide/installation' },
                { text: '命令', link: '/zh/guide/commands' },
                { text: '关于', link: '/zh/guide/about' },
              ],
            },
            {
              text: '配置',
              items: [
                { text: '配置指南', link: '/zh/guide/configuration' },
                { text: 'LLM 提供商', link: '/zh/guide/providers' },
                { text: '自定义提示词', link: '/zh/guide/prompts' },
              ],
            },
            {
              text: '进阶',
              items: [
                { text: 'Git 别名', link: '/zh/guide/aliases' },
                { text: '故障排除', link: '/zh/guide/troubleshooting' },
              ],
            },
          ],
          '/zh/release-notes/': [
            {
              text: '发布说明',
              items: releaseNotesZh,
            },
          ],
        },
      },
    },
  },

  themeConfig: {
    nav: [
      { text: 'Guide', link: '/guide/installation' },
      { text: 'Release Notes', link: releaseNotes[0]?.link || '/release-notes/' },
      { text: 'About', link: '/guide/about' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Installation', link: '/guide/installation' },
            { text: 'Commands', link: '/guide/commands' },
            { text: 'About', link: '/guide/about' },
          ],
        },
        {
          text: 'Configuration',
          items: [
            { text: 'Configuration Guide', link: '/guide/configuration' },
            { text: 'LLM Providers', link: '/guide/providers' },
            { text: 'Custom Prompts', link: '/guide/prompts' },
          ],
        },
        {
          text: 'Advanced',
          items: [
            { text: 'Git Aliases', link: '/guide/aliases' },
            { text: 'Troubleshooting', link: '/guide/troubleshooting' },
          ],
        },
      ],
      '/release-notes/': [
        {
          text: 'Release Notes',
          items: releaseNotes,
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Undertone0809/gcop-rs' },
    ],

    search: {
      provider: 'local',
    },
  },
})
