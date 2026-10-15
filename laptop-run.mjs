// Drives the same page in a desktop browser with Playwright and prints the page's JSON.
// usage: node laptop-run.mjs <chromium|firefox> <orchard-v2|ironwood-v3> <runs> [url]
// The Playwright module path is passed through PLAYWRIGHT_MODULE (the repo's own install).
const [engine = 'chromium', pool = 'ironwood-v3', runs = '5', url = 'http://localhost:8080/'] = process.argv.slice(2);
const pw = await import(process.env.PLAYWRIGHT_MODULE);
const browser = await pw[engine].launch();
const page = await browser.newPage();
await page.goto(url);
await page.fill('#runs', runs);
await page.click(`button[data-pool="${pool}"]`);
await page.waitForFunction(() => document.getElementById('status').textContent === 'done'
  || document.getElementById('status').textContent.startsWith('error'), null, { timeout: 30 * 60 * 1000 });
const text = await page.textContent('#out');
const out = JSON.parse(text);
out.device.browserVersion = browser.version();
console.log(JSON.stringify(out));
await browser.close();
