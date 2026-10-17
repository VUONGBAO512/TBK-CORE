const fs = require('fs');
const path = require('path');

const e2ePath = path.resolve('crates/pc-doctor-core/tests/e2e_suite.rs');
const content = fs.readFileSync(e2ePath, 'utf8');

// Find build_master_registry
const registryMatch = content.match(/pub fn build_master_registry\(\) -> Vec<TestCase> \{([\s\S]*?)\n\s*\}\n/);
if (!registryMatch) {
    console.error('Could not locate build_master_registry()');
    process.exit(1);
}

const regBody = registryMatch[1];
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

console.log(`Total test cases in build_master_registry: ${cases.length}`);

const tierCounts = {};
cases.forEach(c => {
    tierCounts[c.tier] = (tierCounts[c.tier] || 0) + 1;
});
console.log('Tier breakdown in registry:', tierCounts);

// Check uniqueness of IDs
const idSet = new Set();
const duplicateIds = [];
cases.forEach(c => {
    if (idSet.has(c.id)) {
        duplicateIds.push(c.id);
    }
    idSet.add(c.id);
});
console.log(`Unique test IDs: ${idSet.size}, Duplicates: ${duplicateIds.length} ${duplicateIds.join(', ')}`);

// Now verify each test function implementation in content
console.log('\n--- Checking Test Function Implementations ---');
const suspiciousCases = [];

cases.forEach(c => {
    const fnRegex = new RegExp(`fn\\s+${c.test_fn}\\s*\\(\\s*\\)\\s*->\\s*Result<\\(\\),\\s*String>\\s*\\{([\\s\\S]*?)\\n\\}`);
    const fnMatch = content.match(fnRegex);
    if (!fnMatch) {
        suspiciousCases.push({ id: c.id, fn: c.test_fn, reason: 'Function definition not found' });
        return;
    }
    const body = fnMatch[1].trim();
    // Check if body is trivial / facade (e.g. only "Ok(())")
    const lines = body.split('\n').map(l => l.trim()).filter(l => l.length > 0 && !l.startsWith('//'));
    if (lines.length <= 1 && body === 'Ok(())') {
        suspiciousCases.push({ id: c.id, fn: c.test_fn, reason: 'Trivial dummy pass: only Ok(())' });
    }
    // Check if function contains assertions / error returns
    const hasChecks = /assert|return Err|if .*return Err/.test(body);
    if (!hasChecks) {
        suspiciousCases.push({ id: c.id, fn: c.test_fn, reason: 'No assertions or error returns found in body' });
    }
});

console.log(`Suspicious / Facade test cases found: ${suspiciousCases.length}`);
if (suspiciousCases.length > 0) {
    console.log(JSON.stringify(suspiciousCases, null, 2));
} else {
    console.log('ALL test functions have non-trivial implementations with assertions/validations.');
}
