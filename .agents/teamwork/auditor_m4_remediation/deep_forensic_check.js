const fs = require('fs');
const path = require('path');

const e2ePath = path.resolve('crates/pc-doctor-core/tests/e2e_suite.rs');
const content = fs.readFileSync(e2ePath, 'utf8');

const regMatch = content.match(/pub fn build_master_registry\(\) -> Vec<TestCase> \{([\s\S]*?)\n\s*\}\n/);
const regBody = regMatch[1];
const testCaseRegex = /TestCase\s*\{\s*id:\s*"([^"]+)",\s*tier:\s*"([^"]+)",\s*feature:\s*"([^"]+)",\s*description:\s*"([^"]+)",\s*test_fn:\s*([^\s,}]+)\s*\}/g;

const cases = [];
let m;
while ((m = testCaseRegex.exec(regBody)) !== null) {
    cases.push({
        id: m[1],
        tier: m[2],
        feature: m[3],
        description: m[4],
        test_fn: m[5]
    });
}

console.log('--- Deep Forensic Verification of all 197 tests ---');
let invalidCount = 0;

cases.forEach(c => {
    // Find function
    const startIdx = content.indexOf(`fn ${c.test_fn}()`);
    if (startIdx === -1) {
        console.error(`FAIL: ${c.id} function fn ${c.test_fn}() not found`);
        invalidCount++;
        return;
    }
    // Find next function or section start
    let endIdx = content.indexOf('\nfn ', startIdx + 5);
    if (endIdx === -1) endIdx = content.indexOf('\npub fn ', startIdx + 5);
    if (endIdx === -1) endIdx = content.length;

    const fnContent = content.substring(startIdx, endIdx);
    
    // Check lines of code
    const lines = fnContent.split('\n')
        .map(l => l.trim())
        .filter(l => l.length > 0 && !l.startsWith('//') && !l.startsWith('/*'));

    // Check if it's too short (less than 4 lines including signature and closing brace)
    if (lines.length < 4) {
        console.warn(`WARNING: ${c.id} (${c.test_fn}) has only ${lines.length} lines:\n${fnContent}`);
        if (lines.some(l => l === 'Ok(())')) {
            // Check what else is inside
            const codeLines = lines.filter(l => !l.startsWith('fn ') && l !== '{' && l !== '}' && l !== 'Ok(())');
            if (codeLines.length === 0) {
                console.error(`INTEGRITY VIOLATION: ${c.id} has no body other than Ok(())!`);
                invalidCount++;
            }
        }
    }
});

console.log(`Deep forensic check completed. Total invalid/facade tests: ${invalidCount}`);
