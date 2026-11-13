import type { SchematicFile } from './api';

export type TreeDir = { name: string; path: string; dirs: TreeDir[]; files: SchematicFile[]; count: number };

export function buildTree(list: SchematicFile[]): TreeDir {
	const root: TreeDir = { name: '', path: '', dirs: [], files: [], count: 0 };
	for (const f of list) {
		let node = root;
		for (const name of f.relPath.split('/').slice(0, -1)) {
			const path = node.path ? `${node.path}/${name}` : name;
			let child = node.dirs.find((d) => d.name === name);
			if (!child) {
				child = { name, path, dirs: [], files: [], count: 0 };
				node.dirs.push(child);
			}
			child.count++;
			node = child;
		}
		node.files.push(f);
	}
	const sort = (node: TreeDir) => {
		node.dirs.sort((a, b) => a.name.localeCompare(b.name));
		node.files.sort((a, b) => a.name.localeCompare(b.name));
		node.dirs.forEach(sort);
	};
	sort(root);
	return root;
}
