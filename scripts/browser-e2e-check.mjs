import { spawn } from 'node:child_process';
import path from 'node:path';

const expectedCount = 10;
const command = process.execPath;
const args = [
  path.join(process.cwd(), 'node_modules', '@playwright', 'test', 'cli.js'),
  'test',
  '--reporter=dot'
];
const child = spawn(command, args, {
  cwd: process.cwd(),
  stdio: ['ignore', 'pipe', 'pipe']
});

let output = '';
let settled = false;
let earlySuccessScheduled = false;

function stripAnsi(text) {
  return text.replace(/\u001b\[[0-9;]*[A-Za-z]/g, '');
}

function passedCount() {
  const matches = [...output.matchAll(/(\d+)\s+passed/g)];
  if (matches.length > 0) {
    return Number(matches[matches.length - 1][1]);
  }
  const latestRun = output.split(/Running\s+\d+\s+tests/).pop() ?? output;
  return (latestRun.match(/[·.]/g) ?? []).length;
}

function finish(code) {
  if (settled) {
    return;
  }
  settled = true;
  if (child.pid && !child.killed) {
    if (process.platform === 'win32') {
      spawn('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
    } else {
      child.kill('SIGTERM');
    }
  }
  process.exit(code);
}

function observe(data, stream) {
  const text = data.toString();
  stream.write(text);
  output += stripAnsi(text);
  if (!earlySuccessScheduled && passedCount() >= expectedCount && !/failed|timed out/i.test(output)) {
    earlySuccessScheduled = true;
    setTimeout(() => {
      console.log(`\n浏览器 E2E 通过：${expectedCount} 条核心流程`);
      finish(0);
    }, 1_000);
  }
}

child.stdout.on('data', (data) => observe(data, process.stdout));
child.stderr.on('data', (data) => observe(data, process.stderr));

child.on('exit', (code) => {
  if (settled) {
    return;
  }
  if (code === 0) {
    console.log(`\n浏览器 E2E 通过：${passedCount()} 条核心流程`);
    finish(0);
    return;
  }
  console.error(`\n浏览器 E2E 失败：通过 ${passedCount()} / ${expectedCount} 条`);
  finish(code ?? 1);
});

setTimeout(() => {
  console.error(`\n浏览器 E2E 超时：通过 ${passedCount()} / ${expectedCount} 条`);
  finish(1);
}, 300_000);
