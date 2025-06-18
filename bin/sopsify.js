#!/usr/bin/env node
import { program } from 'commander';
import { run } from '../lib/sopsify.js';

program
	.version('1.0.0')
	.option('-t, --templates <FOLDER>', 'A folder containing template files to encrypt')
	.parse();

run(program.opts()).catch(err => {
	console.error(`❌ ${err.message}`);
	process.exit(1);
});

