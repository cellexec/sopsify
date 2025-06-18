import fs from 'fs/promises';
import yaml from 'js-yaml';
import { join } from 'path';
import { execSync } from 'child_process';

export async function main(options = {}) {
	await run(options);
}

export async function run(opts) {
	const templatesPath = opts.templates;

	console.log('🔄 Running pre-checks...');
	await checkRequiredFiles(['.sops.yaml', '.sopsify.yaml']);
	checkSopsInstalled();

	const sopsifyConfig = await loadYamlFile('.sopsify.yaml');
	const templates = await loadTemplateFiles(templatesPath);
	const templateContents = await readTemplates(templatesPath, templates);
	await processTemplates(templateContents, sopsifyConfig);
}

async function checkRequiredFiles(filePaths) {
	for (const path of filePaths) {
		await fs.readFile(path, 'utf-8');
		console.log(`   ✅ \`${path}\` found!`);
	}
}

function checkSopsInstalled() {
	try {
		execSync('sops --version', { stdio: 'pipe' });
		console.log("   ✅ sops is installed\n\n");
	} catch {
		throw new Error('sops is not installed or not in PATH');
	}
}

async function loadYamlFile(path) {
	const content = await fs.readFile(path, 'utf-8');
	return yaml.load(content);
}

async function loadTemplateFiles(templateDir) {
	const entries = await fs.readdir(templateDir, { withFileTypes: true });
	const templates = entries
		.filter(e => e.isFile())
		.filter(e => /\.(ya?ml)$/.test(e.name));

	console.log(`✅ Templates read: ${templates.length}`);
	return templates;
}

async function readTemplates(templatePath, templates) {
	console.log('🔄 Checking Templates');
	const files = {};

	for (const template of templates) {
		const fullPath = join(templatePath, template.name);
		const content = await fs.readFile(fullPath, 'utf-8');
		const parsed = yaml.load(content);

		validateTemplate(parsed, fullPath);
		files[fullPath] = parsed;
	}

	return files;
}

function validateTemplate(template, filePath) {
	if (!template || !template.kind) {
		throw new Error(`Error in '${filePath}': Missing 'kind'`);
	}
	if (template.kind.toLowerCase() !== 'secret') {
		throw new Error(`Error in '${filePath}': Template is not of kind 'Secret'`);
	}
}

async function processTemplates(templates, config) {
	const clusterConfigs = config.sopsify;

	for (const clusterItem of clusterConfigs) {
		const [clusterName, templateDefs] = Object.entries(clusterItem)[0];
		console.log(`\n🌍 Processing cluster: ${clusterName}`);

		const clusterDir = join('clusters', clusterName);
		try {
			const stat = await fs.stat(clusterDir);
			if (!stat.isDirectory()) {
				throw new Error();
			}
		} catch {
			throw new Error(`Cluster folder '${clusterDir}' does not exist or is not a directory. Aborting.`);
		}

		for (const templateDef of templateDefs) {
			const templateName = templateDef.template;
			const values = templateDef.values;

			const templatePath = Object.keys(templates).find(p => p.endsWith(templateName));
			if (!templatePath) {
				console.warn(`   ⚠️ Template file not found for: ${templateName}`);
				continue;
			}

			const originalTemplate = templates[templatePath];
			const keySection = getKeyAccessor(originalTemplate);
			if (!keySection) {
				throw new Error(`Template '${templateName}' must contain 'data' or 'stringData'`);
			}

			// Collect all namespaces from all values (union)
			const allNamespaces = new Set();
			for (const entry of values) {
				// Check for duplicate namespaces inside a single entry
				const nsSet = new Set(entry.namespaces);
				if (nsSet.size !== entry.namespaces.length) {
					throw new Error(
						`Duplicate namespaces detected in key '${entry.key}' for template '${templateName}' in cluster '${clusterName}': ${entry.namespaces}`
					);
				}
				entry.namespaces.forEach(ns => allNamespaces.add(ns));
			}

			// Build nested map: key -> namespace -> value
			const keyNamespaceValueMap = {};
			for (const entry of values) {
				if (!keyNamespaceValueMap[entry.key]) {
					keyNamespaceValueMap[entry.key] = {};
				}

				for (const ns of entry.namespaces) {
					if (keyNamespaceValueMap[entry.key][ns] !== undefined) {
						throw new Error(
							`Duplicate value for key '${entry.key}' in namespace '${ns}' for template '${templateName}' in cluster '${clusterName}'`
						);
					}
					keyNamespaceValueMap[entry.key][ns] = entry.value;
				}
			}

			// Gather placeholders from template section
			const placeholders = new Set();
			const section = originalTemplate[keySection];
			for (const v of Object.values(section)) {
				if (isPlaceholder(v)) {
					const ph = getPlaceholderName(v);
					placeholders.add(ph);
				}
			}

			// Validate all placeholders have values for all namespaces
			for (const ph of placeholders) {
				if (!keyNamespaceValueMap[ph]) {
					throw new Error(
						`❌ Placeholder '${ph}' in template '${templateName}' for cluster '${clusterName}' has no values defined`
					);
				}

				const missingNs = [...allNamespaces].filter(ns => !(ns in keyNamespaceValueMap[ph]));
				if (missingNs.length > 0) {
					throw new Error(
						`❌ Key '${ph}' in template '${templateName}' for cluster '${clusterName}' is missing namespaces: ${missingNs.join(', ')}`
					);
				}
			}

			const usedKeys = new Set();

			for (const ns of allNamespaces) {
				const rendered = JSON.parse(JSON.stringify(originalTemplate)); // deep copy

				// Add metadata.namespace field
				if (!rendered.metadata) {
					rendered.metadata = {};
				}
				rendered.metadata.namespace = ns;

				const section = rendered[keySection];

				for (const [k, v] of Object.entries(section)) {
					if (isPlaceholder(v)) {
						const placeholder = getPlaceholderName(v);
						usedKeys.add(placeholder);
						section[k] = keyNamespaceValueMap[placeholder][ns];
					}
				}

				const targetDir = join(clusterDir, 'secrets', ns);
				await fs.mkdir(targetDir, { recursive: true });

				const plaintextPath = join(targetDir, templateName);
				const encryptedPath = plaintextPath.replace(/\.ya?ml$/, '.enc.yaml');

				console.log(`   🔄 Rendering: ${ns}/${templateName}`);
				await fs.writeFile(plaintextPath, yaml.dump(rendered), 'utf-8');

				execSync(`sops -e -i ${plaintextPath}`);

				await fs.rename(plaintextPath, encryptedPath);
				console.log(`   🔐 Encrypted: ${ns}/${templateName.replace(/\.ya?ml$/, '.enc.yaml')}`);
			}

			// Warn for unused keys
			for (const k of Object.keys(keyNamespaceValueMap)) {
				if (!usedKeys.has(k)) {
					console.warn(`   ⚠️ Warning: key '${k}' is defined in .sopsify.yaml but not used in template '${templateName}'`);
				}
			}
		}
	}
}

function getKeyAccessor(file) {
	if (file.data) return 'data';
	if (file.stringData) return 'stringData';
	return undefined;
}

function isPlaceholder(value) {
	return typeof value === 'string' && /^\$\{[a-zA-Z_][a-zA-Z0-9_-]*\}$/.test(value);
}

function getPlaceholderName(placeholder) {
	const match = placeholder.match(/^\$\{([a-zA-Z_][a-zA-Z0-9_-]*)\}$/);
	return match ? match[1] : null;
}

