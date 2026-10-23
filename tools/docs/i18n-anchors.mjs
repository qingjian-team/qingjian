// 译本里的锚点链接照抄的是中文标题；按「目标页第几个标题」换成译本标题的锚点。
// 标题数对不上、链接的目标页或锚点找不到时逐条列出来。
// 用法：node tools/docs/i18n-anchors.mjs docs/user <语言>（如 en、zh-Hant）
import { readFileSync, writeFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, relative, posix, sep } from 'node:path';

const [root, locale] = process.argv.slice(2);
const tr = join(root, 'i18n', locale);
const headingId = (t) => t.trim().replace(/[\s/?#%&+;,:@=$]+/g, '-');
const toPosix = (p) => p.split(sep).join('/');

function headings(file) {
	const out = [];
	let fence = false;
	for (const line of readFileSync(file, 'utf8').split(/\r?\n/)) {
		if (/^(```|~~~)/.test(line)) fence = !fence;
		if (fence) continue;
		const m = /^(#{2,6})\s+(.*?)\s*$/.exec(line);
		if (m) {
			const text = m[2]
				.replace(/`([^`]*)`/g, '$1')
				.replace(/\*\*|__/g, '')
				.replace(/\[([^\]]*)\]\([^)]*\)/g, '$1');
			out.push(headingId(text));
		}
	}
	return out;
}

const walk = (d) =>
	readdirSync(d).flatMap((n) =>
		statSync(join(d, n)).isDirectory()
			? walk(join(d, n))
			: n.endsWith('.md') && n !== 'README.md'
				? [join(d, n)]
				: []
	);

let fixed = 0;
const problems = [];
for (const file of walk(tr)) {
	const rel = toPosix(relative(tr, file));
	const zhH = headings(join(root, rel));
	const trH = headings(file);
	if (zhH.length !== trH.length) problems.push(`${rel}: 标题数 ${zhH.length} ≠ ${trH.length}`);
	let text = readFileSync(file, 'utf8');
	// 链接目标里允许成对的括号：英文标题的锚点会带 (Windows) 这样的括号
	const link = /\]\(((?:[^()\s]|\([^()\s]*\))*?)#((?:[^()\s]|\([^()\s]*\))+)\)/g;
	text = text.replace(link, (all, path, anchor) => {
		if (/^https?:/.test(path)) return all;
		const targetRel = path ? posix.normalize(posix.join(posix.dirname(rel), path)) : rel;
		const zhT = join(root, targetRel);
		const trT = join(tr, targetRel);
		if (!existsSync(zhT) || !existsSync(trT)) {
			problems.push(`${rel}: 目标不存在 ${path}`);
			return all;
		}
		const zh = headings(zhT);
		const t = headings(trT);
		const decoded = decodeURIComponent(anchor);
		if (t.includes(decoded) && !zh.includes(decoded)) return all;
		const i = zh.indexOf(decoded);
		if (i < 0 || !t[i]) {
			problems.push(`${rel}: 找不到锚点 ${path}#${anchor}`);
			return all;
		}
		if (t[i] !== decoded) fixed++;
		return `](${path}#${t[i]})`;
	});
	writeFileSync(file, text);
}
console.log(`改了 ${fixed} 处`);
for (const p of problems) console.log('!', p);
