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
          {
            text: '命令',
            items: [
              { text: '命令总览', link: '/zh/guide/commands' },
              { text: 'commit', link: '/zh/guide/commands/commit' },
              { text: 'review', link: '/zh/guide/commands/review' },
              { text: 'config', link: '/zh/guide/commands/config' },
              { text: 'hook', link: '/zh/guide/commands/hook' },
            ],
          },
          {
            text: '工作流',
            items: [
              { text: 'Git 别名', link: '/zh/guide/aliases' },
              { text: '故障排除', link: '/zh/guide/troubleshooting' },
              { text: 'Provider 健康检查', link: '/zh/guide/provider-health' },
            ],
          },
          { text: '发布说明', link: releaseNotesZh[0]?.link || '/zh/release-notes/' },
          { text: '关于', link: '/zh/guide/about' },
        ],
        sidebar: {
          '/zh/guide/': [
            {
              text: '入门',
              items: [
                { text: '安装', link: '/zh/guide/installation' },
                { text: '命令总览', link: '/zh/guide/commands' },
                { text: '关于', link: '/zh/guide/about' },
              ],
            },
            {
              text: '命令参考',
              collapsed: true,
              items: [
                {
                  text: '核心命令',
                  collapsed: false,
                  items: [
                    { text: 'init', link: '/zh/guide/commands/init' },
                    { text: 'commit', link: '/zh/guide/commands/commit' },
                    { text: 'review', link: '/zh/guide/commands/review' },
                  ],
                },
                {
                  text: '管理与输出',
                  collapsed: true,
                  items: [
                    { text: 'config', link: '/zh/guide/commands/config' },
                    { text: 'alias', link: '/zh/guide/commands/alias' },
                    { text: 'stats', link: '/zh/guide/commands/stats' },
                    { text: 'hook', link: '/zh/guide/commands/hook' },
                    { text: '自动化与环境', link: '/zh/guide/commands/automation' },
                  ],
                },
              ],
            },
            {
              text: '配置',
              collapsed: true,
              items: [
                { text: '配置指南', link: '/zh/guide/configuration' },
                { text: 'LLM 提供商', link: '/zh/guide/providers' },
                { text: 'Provider 健康检查', link: '/zh/guide/provider-health' },
                { text: '自定义提示词', link: '/zh/guide/prompts' },
              ],
            },
            {
              text: 'Git 别名',
              collapsed: true,
              items: [
                {
                  text: '基础',
                  collapsed: false,
                  items: [
                    { text: '总览', link: '/zh/guide/aliases' },
                    { text: '入门与安装', link: '/zh/guide/aliases/getting-started' },
                  ],
                },
                {
                  text: '常用别名',
                  collapsed: true,
                  items: [
                    { text: '提交类别名', link: '/zh/guide/aliases/commit' },
                    { text: '审查类别名', link: '/zh/guide/aliases/review' },
                    { text: '工具类别名', link: '/zh/guide/aliases/utility' },
                  ],
                },
                {
                  text: '进阶',
                  collapsed: true,
                  items: [
                    { text: '管理与排障', link: '/zh/guide/aliases/operations' },
                    { text: '工作流与最佳实践', link: '/zh/guide/aliases/workflows' },
                  ],
                },
              ],
            },
            {
              text: '故障排除',
              collapsed: true,
              items: [
                { text: '总览', link: '/zh/guide/troubleshooting' },
                {
                  text: '连接与配置',
                  collapsed: false,
                  items: [
                    { text: '安装问题', link: '/zh/guide/troubleshooting/installation' },
                    { text: '配置问题', link: '/zh/guide/troubleshooting/configuration' },
                    { text: 'API 问题', link: '/zh/guide/troubleshooting/api' },
                    { text: '网络问题', link: '/zh/guide/troubleshooting/network' },
                  ],
                },
                {
                  text: '流程与调试',
                  collapsed: true,
                  items: [
                    { text: '审查与 Git 问题', link: '/zh/guide/troubleshooting/review-and-git' },
                    { text: '调试与获取帮助', link: '/zh/guide/troubleshooting/debug-and-support' },
                  ],
                },
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
      {
        text: 'Commands',
        items: [
          { text: 'Command Overview', link: '/guide/commands' },
          { text: 'commit', link: '/guide/commands/commit' },
          { text: 'review', link: '/guide/commands/review' },
          { text: 'config', link: '/guide/commands/config' },
          { text: 'hook', link: '/guide/commands/hook' },
        ],
      },
      {
        text: 'Workflow',
        items: [
          { text: 'Git Aliases', link: '/guide/aliases' },
          { text: 'Troubleshooting', link: '/guide/troubleshooting' },
          { text: 'Provider Health Checks', link: '/guide/provider-health' },
        ],
      },
      { text: 'Release Notes', link: releaseNotes[0]?.link || '/release-notes/' },
      { text: 'About', link: '/guide/about' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Installation', link: '/guide/installation' },
            { text: 'Command Overview', link: '/guide/commands' },
            { text: 'About', link: '/guide/about' },
          ],
        },
        {
          text: 'Command Reference',
          collapsed: true,
          items: [
            {
              text: 'Core Commands',
              collapsed: false,
              items: [
                { text: 'init', link: '/guide/commands/init' },
                { text: 'commit', link: '/guide/commands/commit' },
                { text: 'review', link: '/guide/commands/review' },
              ],
            },
            {
              text: 'Management & Output',
              collapsed: true,
              items: [
                { text: 'config', link: '/guide/commands/config' },
                { text: 'alias', link: '/guide/commands/alias' },
                { text: 'stats', link: '/guide/commands/stats' },
                { text: 'hook', link: '/guide/commands/hook' },
                { text: 'Automation & Env Vars', link: '/guide/commands/automation' },
              ],
            },
          ],
        },
        {
          text: 'Configuration',
          collapsed: true,
          items: [
            { text: 'Configuration Guide', link: '/guide/configuration' },
            { text: 'LLM Providers', link: '/guide/providers' },
            { text: 'Provider Health Checks', link: '/guide/provider-health' },
            { text: 'Custom Prompts', link: '/guide/prompts' },
          ],
        },
        {
          text: 'Git Aliases',
          collapsed: true,
          items: [
            {
              text: 'Basics',
              collapsed: false,
              items: [
                { text: 'Overview', link: '/guide/aliases' },
                { text: 'Getting Started', link: '/guide/aliases/getting-started' },
              ],
            },
            {
              text: 'Common Aliases',
              collapsed: true,
              items: [
                { text: 'Commit Aliases', link: '/guide/aliases/commit' },
                { text: 'Review Aliases', link: '/guide/aliases/review' },
                { text: 'Utility Aliases', link: '/guide/aliases/utility' },
              ],
            },
            {
              text: 'Advanced',
              collapsed: true,
              items: [
                { text: 'Operations', link: '/guide/aliases/operations' },
                { text: 'Workflows & Best Practices', link: '/guide/aliases/workflows' },
              ],
            },
          ],
        },
        {
          text: 'Troubleshooting',
          collapsed: true,
          items: [
            { text: 'Overview', link: '/guide/troubleshooting' },
            {
              text: 'Connection & Config',
              collapsed: false,
              items: [
                { text: 'Installation Issues', link: '/guide/troubleshooting/installation' },
                { text: 'Configuration Issues', link: '/guide/troubleshooting/configuration' },
                { text: 'API Issues', link: '/guide/troubleshooting/api' },
                { text: 'Network Issues', link: '/guide/troubleshooting/network' },
              ],
            },
            {
              text: 'Workflow & Debug',
              collapsed: true,
              items: [
                { text: 'Review and Git Issues', link: '/guide/troubleshooting/review-and-git' },
                { text: 'Debug and Support', link: '/guide/troubleshooting/debug-and-support' },
              ],
            },
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
      { icon: 'github', link: 'https://github.com/AptS-1547/gcop-rs' },
    ],

    search: {
      provider: 'local',
    },
  },
})
