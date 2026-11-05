const fs = require('fs');
const dir = process.argv[2], out = process.argv[3];
const names = {
  play: 'play', pause: 'pause', next: 'skip-forward', previous: 'skip-back', shuffle: 'shuffle',
  repeat: 'repeat', repeatOne: 'repeat-1', volume: 'volume-2', volumeLow: 'volume-1', muted: 'volume-x',
  songs: 'music-2', music: 'music', albums: 'disc-3', artists: 'mic-vocal', playlists: 'list-music',
  queue: 'list-ordered', heart: 'heart', settings: 'settings', search: 'search', plus: 'plus', x: 'x',
  trash: 'trash-2', more: 'ellipsis', folder: 'folder', folderOpen: 'folder-open', back: 'chevron-left',
  chevronRight: 'chevron-right', check: 'check', download: 'download', copy: 'copy', file: 'file',
  image: 'image', clipboard: 'clipboard', note: 'sticky-note', pin: 'pin', pinOff: 'pin-off',
  external: 'external-link', shelf: 'inbox', pencil: 'pencil', clock: 'clock', sort: 'arrow-up-down',
  keyboard: 'keyboard', sun: 'sun', moon: 'moon', monitor: 'monitor', listPlus: 'list-plus',
  listEnd: 'list-end', arrowUp: 'arrow-up', arrowDown: 'arrow-down', fileText: 'file-text', hardDrive: 'hard-drive',
};
let src = `// Generated from lucide-static 1.69.0 (ISC License, Copyright (c) Lucide Icons and
// Contributors; see https://lucide.dev/license). Only the icons Sheen uses are
// included. Each entry is the inner markup of a 24×24, stroke-based SVG.
// Regenerate with: node scripts/gen-icons.cjs <lucide-static>/icons ui/src/shared/icons.ts

export const icons = {\n`;
for (const [key, file] of Object.entries(names)) {
  const svg = fs.readFileSync(`${dir}/${file}.svg`, 'utf8');
  const inner = svg.slice(svg.indexOf('>', svg.indexOf('<svg')) + 1, svg.lastIndexOf('</svg>'))
    .split('\n').map(s => s.trim()).filter(Boolean).join('');
  src += `  ${key}: ${JSON.stringify(inner)},\n`;
}
src += `} as const;\n\nexport type IconName = keyof typeof icons;\n`;
fs.writeFileSync(out, src);
console.log(Object.keys(names).length, 'icons');
