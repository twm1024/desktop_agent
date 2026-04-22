// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

import React, { useState } from 'react';

interface MarketSkill {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  tags: string[];
  downloads: number;
  rating: number;
  verified: boolean;
  featured: boolean;
}

const Market: React.FC = () => {
  const [searchQuery, setSearchQuery] = useState('');
  const [skills] = useState<MarketSkill[]>([
    {
      id: '1',
      name: 'ocr-text',
      version: '1.2.0',
      description: 'OCR文字识别 - 支持中英文图片文字提取',
      author: 'official',
      tags: ['text', 'ocr', 'image'],
      downloads: 15200,
      rating: 4.8,
      verified: true,
      featured: true,
    },
    {
      id: '2',
      name: 'pdf-converter',
      version: '2.0.1',
      description: 'PDF格式转换 - 支持PDF转Word/Excel/PPT',
      author: 'community',
      tags: ['pdf', 'document', 'convert'],
      downloads: 8900,
      rating: 4.5,
      verified: true,
      featured: false,
    },
    {
      id: '3',
      name: 'file-organizer',
      version: '1.0.0',
      description: '智能文件整理 - 根据规则自动分类文件',
      author: 'community',
      tags: ['file', 'organize', 'automate'],
      downloads: 3200,
      rating: 4.2,
      verified: false,
      featured: false,
    },
    {
      id: '4',
      name: 'image-processor',
      version: '1.5.3',
      description: '批量图片处理 - 调整大小、添加水印、格式转换',
      author: 'official',
      tags: ['image', 'process', 'batch'],
      downloads: 12800,
      rating: 4.6,
      verified: true,
      featured: true,
    },
    {
      id: '5',
      name: 'data-extractor',
      version: '1.1.0',
      description: '数据提取工具 - 从网页、文件中提取结构化数据',
      author: 'community',
      tags: ['data', 'extract', 'web'],
      downloads: 5400,
      rating: 4.3,
      verified: false,
      featured: false,
    },
    {
      id: '6',
      name: 'backup-manager',
      version: '2.1.0',
      description: '智能备份管理 - 增量备份、定时备份、云同步',
      author: 'official',
      tags: ['backup', 'sync', 'cloud'],
      downloads: 9100,
      rating: 4.7,
      verified: true,
      featured: true,
    },
  ]);

  const [selectedCategory, setSelectedCategory] = useState('all');

  const categories = [
    { id: 'all', label: '全部', icon: '📦' },
    { id: 'text', label: '文本处理', icon: '📝' },
    { id: 'file', label: '文件操作', icon: '📁' },
    { id: 'image', label: '图片处理', icon: '🖼️' },
    { id: 'data', label: '数据处理', icon: '📊' },
    { id: 'system', label: '系统工具', icon: '⚙️' },
  ];

  const filteredSkills = skills.filter((skill) => {
    const matchesSearch =
      searchQuery === '' ||
      skill.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      skill.description.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory =
      selectedCategory === 'all' ||
      skill.tags.some((tag) => tag.includes(selectedCategory));
    return matchesSearch && matchesCategory;
  });

  const formatDownloads = (count: number): string => {
    if (count >= 10000) return `${(count / 10000).toFixed(1)}万`;
    if (count >= 1000) return `${(count / 1000).toFixed(1)}k`;
    return count.toString();
  };

  return (
    <div className="p-6 max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">技能市场</h1>
        <button className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm">
          刷新市场
        </button>
      </div>

      {/* Search bar */}
      <div className="relative mb-6">
        <input
          type="text"
          placeholder="搜索技能..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full px-4 py-3 pl-10 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-white"
        />
        <svg
          className="absolute left-3 top-3.5 h-5 w-5 text-gray-400"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
          />
        </svg>
      </div>

      {/* Categories */}
      <div className="flex gap-2 mb-6 overflow-x-auto">
        {categories.map((cat) => (
          <button
            key={cat.id}
            onClick={() => setSelectedCategory(cat.id)}
            className={`flex items-center gap-1.5 px-4 py-2 rounded-full text-sm whitespace-nowrap transition-colors ${
              selectedCategory === cat.id
                ? 'bg-blue-600 text-white'
                : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
            }`}
          >
            <span>{cat.icon}</span>
            <span>{cat.label}</span>
          </button>
        ))}
      </div>

      {/* Featured skills */}
      {selectedCategory === 'all' && searchQuery === '' && (
        <div className="mb-8">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">推荐技能</h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {skills
              .filter((s) => s.featured)
              .map((skill) => (
                <div
                  key={skill.id}
                  className="p-4 bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 border border-blue-200 dark:border-blue-800 rounded-lg"
                >
                  <div className="flex items-start justify-between mb-2">
                    <h3 className="font-semibold text-gray-900 dark:text-white">{skill.name}</h3>
                    {skill.verified && (
                      <span className="text-xs bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300 px-2 py-0.5 rounded">
                        已认证
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-3">{skill.description}</p>
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-gray-500 dark:text-gray-400">v{skill.version}</span>
                    <button className="px-3 py-1 bg-blue-600 text-white text-sm rounded hover:bg-blue-700">
                      安装
                    </button>
                  </div>
                </div>
              ))}
          </div>
        </div>
      )}

      {/* All skills grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {filteredSkills.map((skill) => (
          <div
            key={skill.id}
            className="p-4 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg hover:shadow-md transition-shadow"
          >
            <div className="flex items-start justify-between mb-2">
              <div>
                <h3 className="font-semibold text-gray-900 dark:text-white">{skill.name}</h3>
                <span className="text-xs text-gray-500 dark:text-gray-400">
                  by {skill.author} · v{skill.version}
                </span>
              </div>
              {skill.verified && (
                <span className="text-green-500 text-sm">✓</span>
              )}
            </div>

            <p className="text-sm text-gray-600 dark:text-gray-400 mb-3 line-clamp-2">
              {skill.description}
            </p>

            <div className="flex flex-wrap gap-1 mb-3">
              {skill.tags.map((tag) => (
                <span
                  key={tag}
                  className="text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 px-2 py-0.5 rounded"
                >
                  {tag}
                </span>
              ))}
            </div>

            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3 text-xs text-gray-500 dark:text-gray-400">
                <span>⬇ {formatDownloads(skill.downloads)}</span>
                <span>★ {skill.rating}</span>
              </div>
              <button className="px-3 py-1 bg-blue-600 text-white text-sm rounded hover:bg-blue-700">
                安装
              </button>
            </div>
          </div>
        ))}
      </div>

      {filteredSkills.length === 0 && (
        <div className="text-center py-12 text-gray-500 dark:text-gray-400">
          <p className="text-lg mb-2">没有找到匹配的技能</p>
          <p className="text-sm">尝试其他搜索词或分类</p>
        </div>
      )}
    </div>
  );
};

export default Market;
