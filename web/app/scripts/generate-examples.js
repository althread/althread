#!/usr/bin/env node

/**
 * Auto-generate examples.ts from the root examples/ directory
 * This keeps the examples in sync and eliminates duplication
 */

const fs = require('fs');
const path = require('path');

// Paths
const ROOT_EXAMPLES_DIR = path.join(__dirname, '../../../examples');
const OUTPUT_FILE = path.join(__dirname, '../examples/examples.ts');

// Helper function to extract metadata from file content
function extractMetadata(content, fileName) {
  // Default metadata
  let title = fileName.replace('.alt', '').replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase());
  let description = `Example program: ${title}`;
  let tags = ['example'];

  // Try to extract better metadata from comments or content
  const lines = content.split('\n');
  
  // Look for title in comments
  const titleMatch = content.match(/\/\/\s*Title:\s*(.+)/i);
  if (titleMatch) {
    title = titleMatch[1].trim();
  }
  
  // Look for description in comments
  const descMatch = content.match(/\/\/\s*Description:\s*(.+)/i);
  if (descMatch) {
    description = descMatch[1].trim();
  }
  
  // Look for tags in comments
  const tagsMatch = content.match(/\/\/\s*Tags:\s*(.+)/i);
  if (tagsMatch) {
    tags = tagsMatch[1].split(',').map(tag => tag.trim().toLowerCase());
  } else {
    // Auto-detect tags based on content
    const autoTags = [];
    
    if (content.includes('shared {')) autoTags.push('shared', 'concurrency');
    if (content.includes('atomic {')) autoTags.push('atomic');
    if (content.includes('channel ')) autoTags.push('channels', 'communication');
    if (content.includes('program ')) autoTags.push('programs');
    if (content.includes('fn ')) autoTags.push('functions');
    if (content.includes('for ') || content.includes('while ')) autoTags.push('loops');
    if (content.includes('if ')) autoTags.push('conditionals');
    if (content.includes('wait ')) autoTags.push('synchronization');
    if (content.includes('send ') || content.includes('receive ')) autoTags.push('messaging');
    if (content.includes('recursive')) autoTags.push('recursion');
    if (content.includes('fibonacci')) autoTags.push('fibonacci', 'math', 'algorithms');
    if (content.includes('peterson')) autoTags.push('peterson', 'mutex', 'critical-section');
    if (content.includes('election')) autoTags.push('election', 'distributed');
    if (content.includes('ring')) autoTags.push('ring');
    if (content.includes('max(') || content.includes('min(')) autoTags.push('math');
    if (content.includes('sum') || content.includes('+=')) autoTags.push('math');
    
    if (autoTags.length > 0) {
      tags = [...new Set([...tags, ...autoTags])]; // Remove duplicates
    }
  }

  return { title, description, tags };
}

// Generate examples.ts content
function generateExamplesFile() {
  console.log('🔄 Generating examples.ts from root examples directory...');
  
  if (!fs.existsSync(ROOT_EXAMPLES_DIR)) {
    console.error(`❌ Root examples directory not found: ${ROOT_EXAMPLES_DIR}`);
    process.exit(1);
  }

  // Read all .alt files from the examples directory
  const files = fs.readdirSync(ROOT_EXAMPLES_DIR)
    .filter(file => file.endsWith('.alt'))
    .sort();

  if (files.length === 0) {
    console.error('❌ No .alt files found in examples directory');
    process.exit(1);
  }

  console.log(`📁 Found ${files.length} example files`);

  // Generate the examples array
  const examples = files.map(fileName => {
    const filePath = path.join(ROOT_EXAMPLES_DIR, fileName);
    const content = fs.readFileSync(filePath, 'utf8').trim();
    const { title, description, tags } = extractMetadata(content, fileName);

    console.log(`  📄 ${fileName} -> "${title}"`);

    return {
      fileName,
      title,
      description,
      tags,
      content
    };
  });

  // Generate the TypeScript file content
  const tsContent = `// This file contains all the example programs embedded as strings
// Auto-generated from the examples/ directory
// Run 'npm run generate-examples' to regenerate

export interface ExampleInfo {
  fileName: string;
  title: string;
  description: string;
  tags: string[];
  content: string;
}

export const EXAMPLES: ExampleInfo[] = [
${examples.map(example => `  {
    fileName: ${JSON.stringify(example.fileName)},
    title: ${JSON.stringify(example.title)},
    description: ${JSON.stringify(example.description)},
    tags: ${JSON.stringify(example.tags)},
    content: ${JSON.stringify(example.content, null, 2).split('\n').join('\n    ')}
  }`).join(',\n')}
];

// Helper function to search examples by content and metadata
export function searchExamples(query: string): ExampleInfo[] {
  const searchTerm = query.toLowerCase().trim();
  
  if (!searchTerm) {
    return EXAMPLES;
  }
  
  return EXAMPLES.filter(example => {
    // Search in title, description, tags, filename, and content
    return (
      example.title.toLowerCase().includes(searchTerm) ||
      example.description.toLowerCase().includes(searchTerm) ||
      example.fileName.toLowerCase().includes(searchTerm) ||
      example.tags.some(tag => tag.toLowerCase().includes(searchTerm)) ||
      example.content.toLowerCase().includes(searchTerm)
    );
  });
}
`;

  // Ensure the examples directory exists
  const outputDir = path.dirname(OUTPUT_FILE);
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  // Write the generated file
  fs.writeFileSync(OUTPUT_FILE, tsContent);
  
  console.log(`✅ Generated examples.ts with ${examples.length} examples`);
  console.log(`📍 Output: ${OUTPUT_FILE}`);
}

// Run the generator
if (require.main === module) {
  generateExamplesFile();
}

module.exports = { generateExamplesFile };
