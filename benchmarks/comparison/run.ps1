param(
    [int]$Runs = 5,
    [int]$Warmups = 1,
    [string[]]$Languages = @("skepa", "python", "c", "cpp", "java", "rust", "node"),
    [string]$Skepac = ""
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$WorkRoot = Join-Path $PSScriptRoot ".work"
$SrcRoot = Join-Path $WorkRoot "src"
$BinRoot = Join-Path $WorkRoot "bin"
$ResultsRoot = Join-Path $PSScriptRoot "results"
$Stamp = Get-Date -Format "yyyyMMdd_HHmmss"
$ResultDir = Join-Path $ResultsRoot $Stamp

$Cases = @(
    "arith_loop",
    "bitmix",
    "nested_loops",
    "fib_iter",
    "gcd_chain",
    "prime_count",
    "collatz",
    "branch_mix",
    "function_calls",
    "vec_push_sum",
    "matrix_walk",
    "fib_rec",
    "string_scan",
    "bytes_scan",
    "map_count",
    "option_result",
    "file_read"
)

function New-CleanDir($Path) {
    if (Test-Path $Path) {
        Remove-Item -Recurse -Force $Path
    }
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Ensure-Dir($Path) {
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

function Write-TextFile($Path, $Text) {
    Ensure-Dir (Split-Path -Parent $Path)
    $Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($Path, $Text, $Utf8NoBom)
}

function New-DataText {
    $Lines = New-Object System.Collections.Generic.List[string]
    for ($i = 0; $i -lt 20000; $i++) {
        $A = ($i * 17 + 11) % 1000003
        $B = ($i * 31 + 7) % 1000003
        $C = ($i * 43 + 19) % 1000003
        $Lines.Add("row=$i;alpha=$A;beta=$B;gamma=$C;tag=skepa")
    }
    return ($Lines -join "`n")
}

function Find-Tool($Names) {
    foreach ($Name in $Names) {
        $Cmd = Get-Command $Name -ErrorAction SilentlyContinue
        if ($Cmd) {
            return $Cmd.Source
        }
    }
    return $null
}

function Quote-ProcessArg($Arg) {
    $Text = [string]$Arg
    if ($Text -notmatch '[\s"]') {
        return $Text
    }
    return '"' + ($Text -replace '"', '\"') + '"'
}

function Resolve-Skepac {
    if ($Skepac -ne "") {
        return (Resolve-Path $Skepac).Path
    }
    $DebugPath = Join-Path $Root "target\debug\skepac.exe"
    $ReleasePath = Join-Path $Root "target\release\skepac.exe"
    if (Test-Path $ReleasePath) {
        return $ReleasePath
    }
    if (Test-Path $DebugPath) {
        return $DebugPath
    }
    $Cmd = Get-Command "skepac" -ErrorAction SilentlyContinue
    if ($Cmd) {
        return $Cmd.Source
    }
    return $null
}

function Invoke-TimedCommand {
    param(
        [string]$File,
        [string[]]$CommandArgs,
        [string]$WorkingDirectory
    )
    $Psi = [System.Diagnostics.ProcessStartInfo]::new()
    $Psi.FileName = $File
    $Psi.Arguments = (($CommandArgs | ForEach-Object { Quote-ProcessArg $_ }) -join " ")
    $Psi.WorkingDirectory = $WorkingDirectory
    $Psi.RedirectStandardOutput = $true
    $Psi.RedirectStandardError = $true
    $Psi.UseShellExecute = $false

    $Proc = [System.Diagnostics.Process]::new()
    $Proc.StartInfo = $Psi
    $Timer = [System.Diagnostics.Stopwatch]::StartNew()
    [void]$Proc.Start()
    $Stdout = $Proc.StandardOutput.ReadToEnd()
    $Stderr = $Proc.StandardError.ReadToEnd()
    $Proc.WaitForExit()
    $Timer.Stop()

    [pscustomobject]@{
        ExitCode = $Proc.ExitCode
        Stdout = $Stdout.Trim()
        Stderr = $Stderr.Trim()
        Ms = $Timer.Elapsed.TotalMilliseconds
    }
}

function Median($Values) {
    if ($Values.Count -eq 0) {
        return $null
    }
    $Sorted = @($Values | Sort-Object)
    $Mid = [int]($Sorted.Count / 2)
    if (($Sorted.Count % 2) -eq 1) {
        return [double]$Sorted[$Mid]
    }
    return ([double]$Sorted[$Mid - 1] + [double]$Sorted[$Mid]) / 2.0
}

function Average($Values) {
    if ($Values.Count -eq 0) {
        return $null
    }
    $Total = 0.0
    foreach ($Value in $Values) {
        $Total += [double]$Value
    }
    return $Total / $Values.Count
}

function Common-C {
@'
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const int64_t MOD = 1000000007LL;

static int64_t arith_loop(void) {
    int64_t acc = 0;
    for (int64_t i = 0; i < 2000000; i++) {
        acc = (acc + ((i * 17 + 13) % MOD)) % MOD;
        acc = (acc * 3 + (i % 97)) % MOD;
    }
    return acc;
}

static int64_t bitmix(void) {
    int64_t acc = 1234567;
    for (int64_t i = 1; i <= 1000000; i++) {
        int64_t mixed = ((i * 1103515245LL) ^ (i << 7) ^ (acc >> 3)) & 2147483647LL;
        acc = (acc + mixed + (i & 255)) % MOD;
    }
    return acc;
}

static int64_t nested_loops(void) {
    int64_t acc = 0;
    for (int64_t i = 0; i < 1200; i++) {
        int64_t row = (i * 17 + 5) % MOD;
        for (int64_t j = 0; j < 1200; j++) {
            int64_t mixed = (row + j * 13 + (i ^ j)) % MOD;
            acc = (acc + mixed + ((i * j) % 97)) % MOD;
        }
    }
    return acc;
}

static int64_t fib_iter(void) {
    int64_t a = 0;
    int64_t b = 1;
    for (int64_t i = 0; i < 2000000; i++) {
        int64_t c = (a + b) % MOD;
        a = b;
        b = c;
    }
    return b;
}

static int64_t gcd_one(int64_t a, int64_t b) {
    while (b != 0) {
        int64_t t = a % b;
        a = b;
        b = t;
    }
    return a;
}

static int64_t gcd_chain(void) {
    int64_t acc = 0;
    for (int64_t i = 1; i <= 800000; i++) {
        acc = (acc + gcd_one(i * 37 + 11, i * 19 + 7)) % MOD;
    }
    return acc;
}

static int64_t prime_count(void) {
    int64_t count = 0;
    for (int64_t n = 2; n <= 20000; n++) {
        int64_t prime = 1;
        for (int64_t d = 2; d * d <= n; d++) {
            if (n % d == 0) {
                prime = 0;
                break;
            }
        }
        if (prime) {
            count++;
        }
    }
    return count;
}

static int64_t collatz(void) {
    int64_t total = 0;
    for (int64_t n = 1; n <= 200000; n++) {
        int64_t x = n;
        int64_t steps = 0;
        while (x != 1 && steps < 500) {
            if ((x & 1) == 0) {
                x = x / 2;
            } else {
                x = 3 * x + 1;
            }
            steps++;
        }
        total = (total + steps) % MOD;
    }
    return total;
}

static int64_t branch_mix(void) {
    int64_t acc = 0;
    for (int64_t i = 0; i < 1500000; i++) {
        if (i % 3 == 0) {
            acc += i * 2;
        } else if (i % 3 == 1) {
            acc += i + 7;
        } else {
            acc += i ^ 31;
        }
        acc %= MOD;
    }
    return acc;
}

static int64_t tiny_func(int64_t x) {
    return ((x * 31 + 7) ^ (x >> 2)) % MOD;
}

static int64_t function_calls(void) {
    int64_t acc = 0;
    for (int64_t i = 0; i < 1000000; i++) {
        acc = (acc + tiny_func(i)) % MOD;
    }
    return acc;
}

static int64_t vec_push_sum(void) {
    int64_t n = 50000;
    int64_t *values = (int64_t*)malloc(sizeof(int64_t) * (size_t)n);
    int64_t acc = 0;
    for (int64_t i = 0; i < n; i++) {
        values[i] = (i * 7 + 3) % MOD;
    }
    for (int64_t i = 0; i < n; i++) {
        acc = (acc + values[i]) % MOD;
    }
    free(values);
    return acc;
}

static int64_t matrix_walk(void) {
    int64_t acc = 0;
    for (int64_t r = 0; r < 350; r++) {
        for (int64_t c = 0; c < 350; c++) {
            int64_t value = (r * 101 + c * 103 + ((r ^ c) & 255)) % MOD;
            acc = (acc + value) % MOD;
        }
    }
    return acc;
}

static int64_t fib_rec_inner(int64_t n) {
    if (n <= 1) return n;
    return fib_rec_inner(n - 1) + fib_rec_inner(n - 2);
}

static int64_t fib_rec(void) {
    return fib_rec_inner(30);
}

static int64_t string_scan(void) {
    const char *text = "alpha-beta-gamma-delta-skepa-language-benchmark";
    int64_t acc = 0;
    size_t len = strlen(text);
    for (int64_t i = 0; i < 120000; i++) {
        if (strstr(text, "gamma") != NULL) acc += (int64_t)len + (i % 97);
        if (strncmp(text, "alpha", 5) == 0) acc += 11;
        if (strcmp(text + len - 9, "benchmark") == 0) acc += 17;
        acc %= MOD;
    }
    return acc;
}

static int64_t bytes_scan(void) {
    const unsigned char *text = (const unsigned char*)"alpha-beta-gamma-delta-skepa-language-benchmark";
    int64_t acc = 0;
    size_t len = strlen((const char*)text);
    for (int64_t round = 0; round < 90000; round++) {
        for (size_t i = 0; i < len; i++) {
            acc = (acc + text[i] * (int64_t)(i + 1) + round) % MOD;
        }
    }
    return acc;
}

static const char *map_key(int64_t i) {
    switch (i & 7) {
        case 0: return "alpha";
        case 1: return "beta";
        case 2: return "gamma";
        case 3: return "delta";
        case 4: return "epsilon";
        case 5: return "zeta";
        case 6: return "eta";
        default: return "theta";
    }
}

static int64_t map_count(void) {
    int64_t counts[8] = {0,0,0,0,0,0,0,0};
    int64_t acc = 0;
    for (int64_t i = 0; i < 300000; i++) {
        int64_t idx = i & 7;
        counts[idx] += (i % 13) + 1;
        acc = (acc + counts[idx] + (int64_t)strlen(map_key(i))) % MOD;
    }
    return acc;
}

static int64_t option_result(void) {
    int64_t acc = 0;
    for (int64_t i = 1; i <= 80000; i++) {
        if (i % 11 == 0) {
            acc += 3;
        } else {
            int64_t value = i * 7 + 5;
            if (value % 17 == 0) acc += 19;
            else acc = (acc + value) % MOD;
        }
    }
    return acc % MOD;
}

static int64_t file_read(const char *path) {
    FILE *file = fopen(path, "rb");
    if (!file) return -1;
    int64_t acc = 0;
    int ch = 0;
    while ((ch = fgetc(file)) != EOF) {
        acc = (acc * 131 + ch) % MOD;
    }
    fclose(file);
    return acc;
}

int main(int argc, char **argv) {
    if (argc < 2) return 2;
    const char *name = argv[1];
    const char *data_path = argc >= 3 ? argv[2] : "";
    int64_t result = 0;
    if (strcmp(name, "arith_loop") == 0) result = arith_loop();
    else if (strcmp(name, "bitmix") == 0) result = bitmix();
    else if (strcmp(name, "nested_loops") == 0) result = nested_loops();
    else if (strcmp(name, "fib_iter") == 0) result = fib_iter();
    else if (strcmp(name, "gcd_chain") == 0) result = gcd_chain();
    else if (strcmp(name, "prime_count") == 0) result = prime_count();
    else if (strcmp(name, "collatz") == 0) result = collatz();
    else if (strcmp(name, "branch_mix") == 0) result = branch_mix();
    else if (strcmp(name, "function_calls") == 0) result = function_calls();
    else if (strcmp(name, "vec_push_sum") == 0) result = vec_push_sum();
    else if (strcmp(name, "matrix_walk") == 0) result = matrix_walk();
    else if (strcmp(name, "fib_rec") == 0) result = fib_rec();
    else if (strcmp(name, "string_scan") == 0) result = string_scan();
    else if (strcmp(name, "bytes_scan") == 0) result = bytes_scan();
    else if (strcmp(name, "map_count") == 0) result = map_count();
    else if (strcmp(name, "option_result") == 0) result = option_result();
    else if (strcmp(name, "file_read") == 0) result = file_read(data_path);
    else return 3;
    printf("%lld\n", (long long)result);
    return 0;
}
'@
}

function Common-Cpp {
    (Common-C) -replace '#include <stdint.h>', '#include <cstdint>' -replace '#include <stdio.h>', '#include <cstdio>' -replace '#include <stdlib.h>', '#include <cstdlib>' -replace '#include <string.h>', '#include <cstring>'
}

function Common-Java {
@'
public class Bench {
    static final long MOD = 1000000007L;

    static long arithLoop() {
        long acc = 0;
        for (long i = 0; i < 2000000L; i++) {
            acc = (acc + ((i * 17 + 13) % MOD)) % MOD;
            acc = (acc * 3 + (i % 97)) % MOD;
        }
        return acc;
    }

    static long bitmix() {
        long acc = 1234567L;
        for (long i = 1; i <= 1000000L; i++) {
            long mixed = ((i * 1103515245L) ^ (i << 7) ^ (acc >> 3)) & 2147483647L;
            acc = (acc + mixed + (i & 255)) % MOD;
        }
        return acc;
    }

    static long nestedLoops() {
        long acc = 0;
        for (long i = 0; i < 1200L; i++) {
            long row = (i * 17 + 5) % MOD;
            for (long j = 0; j < 1200L; j++) {
                long mixed = (row + j * 13 + (i ^ j)) % MOD;
                acc = (acc + mixed + ((i * j) % 97)) % MOD;
            }
        }
        return acc;
    }

    static long fibIter() {
        long a = 0;
        long b = 1;
        for (long i = 0; i < 2000000L; i++) {
            long c = (a + b) % MOD;
            a = b;
            b = c;
        }
        return b;
    }

    static long gcdOne(long a, long b) {
        while (b != 0) {
            long t = a % b;
            a = b;
            b = t;
        }
        return a;
    }

    static long gcdChain() {
        long acc = 0;
        for (long i = 1; i <= 800000L; i++) {
            acc = (acc + gcdOne(i * 37 + 11, i * 19 + 7)) % MOD;
        }
        return acc;
    }

    static long primeCount() {
        long count = 0;
        for (long n = 2; n <= 20000L; n++) {
            boolean prime = true;
            for (long d = 2; d * d <= n; d++) {
                if (n % d == 0) {
                    prime = false;
                    break;
                }
            }
            if (prime) count++;
        }
        return count;
    }

    static long collatz() {
        long total = 0;
        for (long n = 1; n <= 200000L; n++) {
            long x = n;
            long steps = 0;
            while (x != 1 && steps < 500) {
                if ((x & 1) == 0) x = x / 2;
                else x = 3 * x + 1;
                steps++;
            }
            total = (total + steps) % MOD;
        }
        return total;
    }

    static long branchMix() {
        long acc = 0;
        for (long i = 0; i < 1500000L; i++) {
            if (i % 3 == 0) acc += i * 2;
            else if (i % 3 == 1) acc += i + 7;
            else acc += i ^ 31;
            acc %= MOD;
        }
        return acc;
    }

    static long tinyFunc(long x) {
        return ((x * 31 + 7) ^ (x >> 2)) % MOD;
    }

    static long functionCalls() {
        long acc = 0;
        for (long i = 0; i < 1000000L; i++) {
            acc = (acc + tinyFunc(i)) % MOD;
        }
        return acc;
    }

    static long vecPushSum() {
        int n = 50000;
        long[] values = new long[n];
        long acc = 0;
        for (int i = 0; i < n; i++) values[i] = (i * 7L + 3) % MOD;
        for (int i = 0; i < n; i++) acc = (acc + values[i]) % MOD;
        return acc;
    }

    static long matrixWalk() {
        long acc = 0;
        for (long r = 0; r < 350L; r++) {
            for (long c = 0; c < 350L; c++) {
                long value = (r * 101 + c * 103 + ((r ^ c) & 255)) % MOD;
                acc = (acc + value) % MOD;
            }
        }
        return acc;
    }

    static long fibRecInner(long n) {
        if (n <= 1) return n;
        return fibRecInner(n - 1) + fibRecInner(n - 2);
    }

    static long fibRec() {
        return fibRecInner(30);
    }

    static long stringScan() {
        String text = "alpha-beta-gamma-delta-skepa-language-benchmark";
        long acc = 0;
        for (long i = 0; i < 120000L; i++) {
            if (text.contains("gamma")) acc += text.length() + (i % 97);
            if (text.startsWith("alpha")) acc += 11;
            if (text.endsWith("benchmark")) acc += 17;
            acc %= MOD;
        }
        return acc;
    }

    static long bytesScan() {
        byte[] data = "alpha-beta-gamma-delta-skepa-language-benchmark".getBytes(java.nio.charset.StandardCharsets.UTF_8);
        long acc = 0;
        for (long round = 0; round < 90000L; round++) {
            for (int i = 0; i < data.length; i++) {
                acc = (acc + ((long)(data[i] & 255)) * (i + 1) + round) % MOD;
            }
        }
        return acc;
    }

    static String mapKey(long i) {
        switch ((int)(i & 7)) {
            case 0: return "alpha";
            case 1: return "beta";
            case 2: return "gamma";
            case 3: return "delta";
            case 4: return "epsilon";
            case 5: return "zeta";
            case 6: return "eta";
            default: return "theta";
        }
    }

    static long mapCount() {
        java.util.HashMap<String, Long> counts = new java.util.HashMap<>();
        long acc = 0;
        for (long i = 0; i < 300000L; i++) {
            String key = mapKey(i);
            long next = counts.getOrDefault(key, 0L) + (i % 13) + 1;
            counts.put(key, next);
            acc = (acc + next + key.length()) % MOD;
        }
        return acc;
    }

    static long optionResult() {
        long acc = 0;
        for (long i = 1; i <= 80000L; i++) {
            if (i % 11 == 0) {
                acc += 3;
            } else {
                long value = i * 7 + 5;
                if (value % 17 == 0) acc += 19;
                else acc = (acc + value) % MOD;
            }
        }
        return acc % MOD;
    }

    static long fileRead(String path) throws java.io.IOException {
        byte[] data = java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(path));
        long acc = 0;
        for (byte raw : data) {
            acc = (acc * 131 + (raw & 255)) % MOD;
        }
        return acc;
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 1) System.exit(2);
        String dataPath = args.length >= 2 ? args[1] : "";
        long result;
        switch (args[0]) {
            case "arith_loop": result = arithLoop(); break;
            case "bitmix": result = bitmix(); break;
            case "nested_loops": result = nestedLoops(); break;
            case "fib_iter": result = fibIter(); break;
            case "gcd_chain": result = gcdChain(); break;
            case "prime_count": result = primeCount(); break;
            case "collatz": result = collatz(); break;
            case "branch_mix": result = branchMix(); break;
            case "function_calls": result = functionCalls(); break;
            case "vec_push_sum": result = vecPushSum(); break;
            case "matrix_walk": result = matrixWalk(); break;
            case "fib_rec": result = fibRec(); break;
            case "string_scan": result = stringScan(); break;
            case "bytes_scan": result = bytesScan(); break;
            case "map_count": result = mapCount(); break;
            case "option_result": result = optionResult(); break;
            case "file_read": result = fileRead(dataPath); break;
            default: System.exit(3); return;
        }
        System.out.println(result);
    }
}
'@
}

function Common-Rust {
@'
const MOD: i64 = 1_000_000_007;

fn arith_loop() -> i64 {
    let mut acc = 0i64;
    for i in 0..2_000_000i64 {
        acc = (acc + ((i * 17 + 13) % MOD)) % MOD;
        acc = (acc * 3 + (i % 97)) % MOD;
    }
    acc
}

fn bitmix() -> i64 {
    let mut acc = 1_234_567i64;
    for i in 1..=1_000_000i64 {
        let mixed = ((i * 1_103_515_245) ^ (i << 7) ^ (acc >> 3)) & 2_147_483_647;
        acc = (acc + mixed + (i & 255)) % MOD;
    }
    acc
}

fn nested_loops() -> i64 {
    let mut acc = 0i64;
    for i in 0..1200i64 {
        let row = (i * 17 + 5) % MOD;
        for j in 0..1200i64 {
            let mixed = (row + j * 13 + (i ^ j)) % MOD;
            acc = (acc + mixed + ((i * j) % 97)) % MOD;
        }
    }
    acc
}

fn fib_iter() -> i64 {
    let mut a = 0i64;
    let mut b = 1i64;
    for _ in 0..2_000_000i64 {
        let c = (a + b) % MOD;
        a = b;
        b = c;
    }
    b
}

fn gcd_one(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn gcd_chain() -> i64 {
    let mut acc = 0i64;
    for i in 1..=800_000i64 {
        acc = (acc + gcd_one(i * 37 + 11, i * 19 + 7)) % MOD;
    }
    acc
}

fn prime_count() -> i64 {
    let mut count = 0i64;
    for n in 2..=20_000i64 {
        let mut prime = true;
        let mut d = 2i64;
        while d * d <= n {
            if n % d == 0 {
                prime = false;
                break;
            }
            d += 1;
        }
        if prime {
            count += 1;
        }
    }
    count
}

fn collatz() -> i64 {
    let mut total = 0i64;
    for n in 1..=200_000i64 {
        let mut x = n;
        let mut steps = 0i64;
        while x != 1 && steps < 500 {
            if (x & 1) == 0 {
                x /= 2;
            } else {
                x = 3 * x + 1;
            }
            steps += 1;
        }
        total = (total + steps) % MOD;
    }
    total
}

fn branch_mix() -> i64 {
    let mut acc = 0i64;
    for i in 0..1_500_000i64 {
        if i % 3 == 0 {
            acc += i * 2;
        } else if i % 3 == 1 {
            acc += i + 7;
        } else {
            acc += i ^ 31;
        }
        acc %= MOD;
    }
    acc
}

fn tiny_func(x: i64) -> i64 {
    ((x * 31 + 7) ^ (x >> 2)) % MOD
}

fn function_calls() -> i64 {
    let mut acc = 0i64;
    for i in 0..1_000_000i64 {
        acc = (acc + tiny_func(i)) % MOD;
    }
    acc
}

fn vec_push_sum() -> i64 {
    let n = 50_000usize;
    let mut values = Vec::with_capacity(n);
    let mut acc = 0i64;
    for i in 0..n {
        values.push((i as i64 * 7 + 3) % MOD);
    }
    for value in values {
        acc = (acc + value) % MOD;
    }
    acc
}

fn matrix_walk() -> i64 {
    let mut acc = 0i64;
    for r in 0..350i64 {
        for c in 0..350i64 {
            let value = (r * 101 + c * 103 + ((r ^ c) & 255)) % MOD;
            acc = (acc + value) % MOD;
        }
    }
    acc
}

fn fib_rec_inner(n: i64) -> i64 {
    if n <= 1 { n } else { fib_rec_inner(n - 1) + fib_rec_inner(n - 2) }
}

fn fib_rec() -> i64 {
    fib_rec_inner(30)
}

fn string_scan() -> i64 {
    let text = "alpha-beta-gamma-delta-skepa-language-benchmark";
    let mut acc = 0i64;
    for i in 0..120_000i64 {
        if text.contains("gamma") {
            acc += text.len() as i64 + (i % 97);
        }
        if text.starts_with("alpha") {
            acc += 11;
        }
        if text.ends_with("benchmark") {
            acc += 17;
        }
        acc %= MOD;
    }
    acc
}

fn bytes_scan() -> i64 {
    let data = b"alpha-beta-gamma-delta-skepa-language-benchmark";
    let mut acc = 0i64;
    for round in 0..90_000i64 {
        for (i, value) in data.iter().enumerate() {
            acc = (acc + (*value as i64) * (i as i64 + 1) + round) % MOD;
        }
    }
    acc
}

fn map_key(i: i64) -> &'static str {
    match i & 7 {
        0 => "alpha",
        1 => "beta",
        2 => "gamma",
        3 => "delta",
        4 => "epsilon",
        5 => "zeta",
        6 => "eta",
        _ => "theta",
    }
}

fn map_count() -> i64 {
    let mut counts = std::collections::HashMap::<&'static str, i64>::new();
    let mut acc = 0i64;
    for i in 0..300_000i64 {
        let key = map_key(i);
        let next = counts.get(key).copied().unwrap_or(0) + (i % 13) + 1;
        counts.insert(key, next);
        acc = (acc + next + key.len() as i64) % MOD;
    }
    acc
}

fn option_result() -> i64 {
    let mut acc = 0i64;
    for i in 1..=80_000i64 {
        let maybe = if i % 11 == 0 { None } else { Some(i * 7 + 5) };
        match maybe {
            None => acc += 3,
            Some(value) => {
                let result: Result<i64, i64> = if value % 17 == 0 { Err(19) } else { Ok(value) };
                match result {
                    Ok(v) => acc = (acc + v) % MOD,
                    Err(e) => acc += e,
                }
            }
        }
    }
    acc % MOD
}

fn file_read(path: &str) -> i64 {
    let data = std::fs::read(path).expect("read benchmark data");
    let mut acc = 0i64;
    for value in data {
        acc = (acc * 131 + value as i64) % MOD;
    }
    acc
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        std::process::exit(2);
    }
    let result = match args[1].as_str() {
        "arith_loop" => arith_loop(),
        "bitmix" => bitmix(),
        "nested_loops" => nested_loops(),
        "fib_iter" => fib_iter(),
        "gcd_chain" => gcd_chain(),
        "prime_count" => prime_count(),
        "collatz" => collatz(),
        "branch_mix" => branch_mix(),
        "function_calls" => function_calls(),
        "vec_push_sum" => vec_push_sum(),
        "matrix_walk" => matrix_walk(),
        "fib_rec" => fib_rec(),
        "string_scan" => string_scan(),
        "bytes_scan" => bytes_scan(),
        "map_count" => map_count(),
        "option_result" => option_result(),
        "file_read" => file_read(args.get(2).map(String::as_str).unwrap_or("")),
        _ => std::process::exit(3),
    };
    println!("{}", result);
}
'@
}

function Common-Python {
@'
import sys

MOD = 1_000_000_007

def arith_loop():
    acc = 0
    for i in range(2_000_000):
        acc = (acc + ((i * 17 + 13) % MOD)) % MOD
        acc = (acc * 3 + (i % 97)) % MOD
    return acc

def bitmix():
    acc = 1_234_567
    for i in range(1, 1_000_001):
        mixed = ((i * 1_103_515_245) ^ (i << 7) ^ (acc >> 3)) & 2_147_483_647
        acc = (acc + mixed + (i & 255)) % MOD
    return acc

def nested_loops():
    acc = 0
    for i in range(1200):
        row = (i * 17 + 5) % MOD
        for j in range(1200):
            mixed = (row + j * 13 + (i ^ j)) % MOD
            acc = (acc + mixed + ((i * j) % 97)) % MOD
    return acc

def fib_iter():
    a = 0
    b = 1
    for _ in range(2_000_000):
        c = (a + b) % MOD
        a = b
        b = c
    return b

def gcd_one(a, b):
    while b != 0:
        t = a % b
        a = b
        b = t
    return a

def gcd_chain():
    acc = 0
    for i in range(1, 800_001):
        acc = (acc + gcd_one(i * 37 + 11, i * 19 + 7)) % MOD
    return acc

def prime_count():
    count = 0
    for n in range(2, 20_001):
        prime = True
        d = 2
        while d * d <= n:
            if n % d == 0:
                prime = False
                break
            d += 1
        if prime:
            count += 1
    return count

def collatz():
    total = 0
    for n in range(1, 200_001):
        x = n
        steps = 0
        while x != 1 and steps < 500:
            if (x & 1) == 0:
                x //= 2
            else:
                x = 3 * x + 1
            steps += 1
        total = (total + steps) % MOD
    return total

def branch_mix():
    acc = 0
    for i in range(1_500_000):
        if i % 3 == 0:
            acc += i * 2
        elif i % 3 == 1:
            acc += i + 7
        else:
            acc += i ^ 31
        acc %= MOD
    return acc

def tiny_func(x):
    return ((x * 31 + 7) ^ (x >> 2)) % MOD

def function_calls():
    acc = 0
    for i in range(1_000_000):
        acc = (acc + tiny_func(i)) % MOD
    return acc

def vec_push_sum():
    values = []
    acc = 0
    for i in range(50_000):
        values.append((i * 7 + 3) % MOD)
    for value in values:
        acc = (acc + value) % MOD
    return acc

def matrix_walk():
    acc = 0
    for r in range(350):
        for c in range(350):
            value = (r * 101 + c * 103 + ((r ^ c) & 255)) % MOD
            acc = (acc + value) % MOD
    return acc

def fib_rec_inner(n):
    if n <= 1:
        return n
    return fib_rec_inner(n - 1) + fib_rec_inner(n - 2)

def fib_rec():
    return fib_rec_inner(30)

def string_scan():
    text = "alpha-beta-gamma-delta-skepa-language-benchmark"
    acc = 0
    for i in range(120_000):
        if "gamma" in text:
            acc += len(text) + (i % 97)
        if text.startswith("alpha"):
            acc += 11
        if text.endswith("benchmark"):
            acc += 17
        acc %= MOD
    return acc

def bytes_scan():
    data = b"alpha-beta-gamma-delta-skepa-language-benchmark"
    acc = 0
    for round_id in range(90_000):
        for i, value in enumerate(data):
            acc = (acc + value * (i + 1) + round_id) % MOD
    return acc

def map_key(i):
    keys = ("alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta")
    return keys[i & 7]

def map_count():
    counts = {}
    acc = 0
    for i in range(300_000):
        key = map_key(i)
        next_value = counts.get(key, 0) + (i % 13) + 1
        counts[key] = next_value
        acc = (acc + next_value + len(key)) % MOD
    return acc

def option_result():
    acc = 0
    for i in range(1, 80_001):
        if i % 11 == 0:
            acc += 3
        else:
            value = i * 7 + 5
            if value % 17 == 0:
                acc += 19
            else:
                acc = (acc + value) % MOD
    return acc % MOD

def file_read(path):
    with open(path, "rb") as handle:
        data = handle.read()
    acc = 0
    for value in data:
        acc = (acc * 131 + value) % MOD
    return acc

CASES = {
    "arith_loop": arith_loop,
    "bitmix": bitmix,
    "nested_loops": nested_loops,
    "fib_iter": fib_iter,
    "gcd_chain": gcd_chain,
    "prime_count": prime_count,
    "collatz": collatz,
    "branch_mix": branch_mix,
    "function_calls": function_calls,
    "vec_push_sum": vec_push_sum,
    "matrix_walk": matrix_walk,
    "fib_rec": fib_rec,
    "string_scan": string_scan,
    "bytes_scan": bytes_scan,
    "map_count": map_count,
    "option_result": option_result,
    "file_read": lambda: file_read(sys.argv[2] if len(sys.argv) > 2 else ""),
}

if len(sys.argv) < 2 or sys.argv[1] not in CASES:
    sys.exit(2)
print(CASES[sys.argv[1]]())
'@
}

function Common-Node {
@'
const MOD = 1000000007;

function arithLoop() {
  let acc = 0;
  for (let i = 0; i < 2000000; i++) {
    acc = (acc + ((i * 17 + 13) % MOD)) % MOD;
    acc = (acc * 3 + (i % 97)) % MOD;
  }
  return Math.trunc(acc);
}

function bitmix() {
  let acc = 1234567;
  for (let i = 1; i <= 1000000; i++) {
    const mixed = ((i * 1103515245) ^ (i << 7) ^ (acc >> 3)) & 2147483647;
    acc = (acc + mixed + (i & 255)) % MOD;
  }
  return Math.trunc(acc);
}

function nestedLoops() {
  let acc = 0;
  for (let i = 0; i < 1200; i++) {
    const row = (i * 17 + 5) % MOD;
    for (let j = 0; j < 1200; j++) {
      const mixed = (row + j * 13 + (i ^ j)) % MOD;
      acc = (acc + mixed + ((i * j) % 97)) % MOD;
    }
  }
  return Math.trunc(acc);
}

function fibIter() {
  let a = 0;
  let b = 1;
  for (let i = 0; i < 2000000; i++) {
    const c = (a + b) % MOD;
    a = b;
    b = c;
  }
  return Math.trunc(b);
}

function gcdOne(a, b) {
  while (b !== 0) {
    const t = a % b;
    a = b;
    b = t;
  }
  return a;
}

function gcdChain() {
  let acc = 0;
  for (let i = 1; i <= 800000; i++) {
    acc = (acc + gcdOne(i * 37 + 11, i * 19 + 7)) % MOD;
  }
  return Math.trunc(acc);
}

function primeCount() {
  let count = 0;
  for (let n = 2; n <= 20000; n++) {
    let prime = true;
    for (let d = 2; d * d <= n; d++) {
      if (n % d === 0) {
        prime = false;
        break;
      }
    }
    if (prime) count++;
  }
  return count;
}

function collatz() {
  let total = 0;
  for (let n = 1; n <= 200000; n++) {
    let x = n;
    let steps = 0;
    while (x !== 1 && steps < 500) {
      if ((x & 1) === 0) x = Math.trunc(x / 2);
      else x = 3 * x + 1;
      steps++;
    }
    total = (total + steps) % MOD;
  }
  return Math.trunc(total);
}

function branchMix() {
  let acc = 0;
  for (let i = 0; i < 1500000; i++) {
    if (i % 3 === 0) acc += i * 2;
    else if (i % 3 === 1) acc += i + 7;
    else acc += i ^ 31;
    acc %= MOD;
  }
  return Math.trunc(acc);
}

function tinyFunc(x) {
  return ((x * 31 + 7) ^ (x >> 2)) % MOD;
}

function functionCalls() {
  let acc = 0;
  for (let i = 0; i < 1000000; i++) {
    acc = (acc + tinyFunc(i)) % MOD;
  }
  return Math.trunc(acc);
}

function vecPushSum() {
  const values = [];
  let acc = 0;
  for (let i = 0; i < 50000; i++) values.push((i * 7 + 3) % MOD);
  for (const value of values) acc = (acc + value) % MOD;
  return Math.trunc(acc);
}

function matrixWalk() {
  let acc = 0;
  for (let r = 0; r < 350; r++) {
    for (let c = 0; c < 350; c++) {
      const value = (r * 101 + c * 103 + ((r ^ c) & 255)) % MOD;
      acc = (acc + value) % MOD;
    }
  }
  return Math.trunc(acc);
}

function fibRecInner(n) {
  if (n <= 1) return n;
  return fibRecInner(n - 1) + fibRecInner(n - 2);
}

function fibRec() {
  return fibRecInner(30);
}

function stringScan() {
  const text = "alpha-beta-gamma-delta-skepa-language-benchmark";
  let acc = 0;
  for (let i = 0; i < 120000; i++) {
    if (text.includes("gamma")) acc += text.length + (i % 97);
    if (text.startsWith("alpha")) acc += 11;
    if (text.endsWith("benchmark")) acc += 17;
    acc %= MOD;
  }
  return Math.trunc(acc);
}

function bytesScan() {
  const data = Buffer.from("alpha-beta-gamma-delta-skepa-language-benchmark", "utf8");
  let acc = 0;
  for (let round = 0; round < 90000; round++) {
    for (let i = 0; i < data.length; i++) {
      acc = (acc + data[i] * (i + 1) + round) % MOD;
    }
  }
  return Math.trunc(acc);
}

function mapKey(i) {
  const keys = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta"];
  return keys[i & 7];
}

function mapCount() {
  const counts = new Map();
  let acc = 0;
  for (let i = 0; i < 300000; i++) {
    const key = mapKey(i);
    const next = (counts.get(key) || 0) + (i % 13) + 1;
    counts.set(key, next);
    acc = (acc + next + key.length) % MOD;
  }
  return Math.trunc(acc);
}

function optionResult() {
  let acc = 0;
  for (let i = 1; i <= 80000; i++) {
    if (i % 11 === 0) {
      acc += 3;
    } else {
      const value = i * 7 + 5;
      if (value % 17 === 0) acc += 19;
      else acc = (acc + value) % MOD;
    }
  }
  return Math.trunc(acc % MOD);
}

function fileRead(path) {
  const data = require("fs").readFileSync(path);
  let acc = 0;
  for (const value of data) {
    acc = (acc * 131 + value) % MOD;
  }
  return Math.trunc(acc);
}

const cases = {
  arith_loop: arithLoop,
  bitmix,
  nested_loops: nestedLoops,
  fib_iter: fibIter,
  gcd_chain: gcdChain,
  prime_count: primeCount,
  collatz,
  branch_mix: branchMix,
  function_calls: functionCalls,
  vec_push_sum: vecPushSum,
  matrix_walk: matrixWalk,
  fib_rec: fibRec,
  string_scan: stringScan,
  bytes_scan: bytesScan,
  map_count: mapCount,
  option_result: optionResult,
  file_read: () => fileRead(process.argv[3] || ""),
};

const name = process.argv[2];
if (!cases[name]) process.exit(2);
console.log(cases[name]().toString());
'@
}

function Common-Skepa {
@'
import io;
import option;
import os;
import str;
import vec;

fn arithLoop() -> Int {
  let acc = 0;
  let i = 0;
  while (i < 2000000) {
    acc = (acc + ((i * 17 + 13) % 1000000007)) % 1000000007;
    acc = (acc * 3 + (i % 97)) % 1000000007;
    i = i + 1;
  }
  return acc;
}

fn bitmix() -> Int {
  let acc = 1234567;
  let i = 1;
  while (i <= 1000000) {
    let mixed = ((i * 1103515245) ^ (i << 7) ^ (acc >> 3)) & 2147483647;
    acc = (acc + mixed + (i & 255)) % 1000000007;
    i = i + 1;
  }
  return acc;
}

fn nestedLoops() -> Int {
  let acc = 0;
  let i = 0;
  while (i < 1200) {
    let row = (i * 17 + 5) % 1000000007;
    let j = 0;
    while (j < 1200) {
      let mixed = (row + j * 13 + (i ^ j)) % 1000000007;
      acc = (acc + mixed + ((i * j) % 97)) % 1000000007;
      j = j + 1;
    }
    i = i + 1;
  }
  return acc;
}

fn fibIter() -> Int {
  let a = 0;
  let b = 1;
  let i = 0;
  while (i < 2000000) {
    let c = (a + b) % 1000000007;
    a = b;
    b = c;
    i = i + 1;
  }
  return b;
}

fn gcdOne(a0: Int, b0: Int) -> Int {
  let a = a0;
  let b = b0;
  while (b != 0) {
    let t = a % b;
    a = b;
    b = t;
  }
  return a;
}

fn gcdChain() -> Int {
  let acc = 0;
  let i = 1;
  while (i <= 800000) {
    acc = (acc + gcdOne(i * 37 + 11, i * 19 + 7)) % 1000000007;
    i = i + 1;
  }
  return acc;
}

fn primeCount() -> Int {
  let count = 0;
  let n = 2;
  while (n <= 20000) {
    let prime = true;
    let d = 2;
    while (d * d <= n) {
      if (n % d == 0) {
        prime = false;
      }
      d = d + 1;
    }
    if (prime) {
      count = count + 1;
    }
    n = n + 1;
  }
  return count;
}

fn collatz() -> Int {
  let total = 0;
  let n = 1;
  while (n <= 200000) {
    let x = n;
    let steps = 0;
    while (x != 1) {
      if ((x & 1) == 0) {
        x = x / 2;
      } else {
        x = 3 * x + 1;
      }
      steps = steps + 1;
    }
    total = (total + steps) % 1000000007;
    n = n + 1;
  }
  return total;
}

fn branchMix() -> Int {
  let acc = 0;
  let i = 0;
  while (i < 1500000) {
    if (i % 3 == 0) {
      acc = acc + i * 2;
    } else if (i % 3 == 1) {
      acc = acc + i + 7;
    } else {
      acc = acc + (i ^ 31);
    }
    acc = acc % 1000000007;
    i = i + 1;
  }
  return acc;
}

fn tinyFunc(x: Int) -> Int {
  return ((x * 31 + 7) ^ (x >> 2)) % 1000000007;
}

fn functionCalls() -> Int {
  let acc = 0;
  let i = 0;
  while (i < 1000000) {
    acc = (acc + tinyFunc(i)) % 1000000007;
    i = i + 1;
  }
  return acc;
}

fn vecPushSum() -> Int {
  let values: Vec[Int] = vec.new();
  let acc = 0;
  let i = 0;
  while (i < 50000) {
    vec.push(values, (i * 7 + 3) % 1000000007);
    i = i + 1;
  }
  let j = 0;
  while (j < vec.len(values)) {
    acc = (acc + option.unwrapSome(vec.get(values, j))) % 1000000007;
    j = j + 1;
  }
  return acc;
}

fn matrixWalk() -> Int {
  let acc = 0;
  let r = 0;
  while (r < 350) {
    let c = 0;
    while (c < 350) {
      let value = (r * 101 + c * 103 + ((r ^ c) & 255)) % 1000000007;
      acc = (acc + value) % 1000000007;
      c = c + 1;
    }
    r = r + 1;
  }
  return acc;
}

fn fibRecInner(n: Int) -> Int {
  let result = n;
  if (n > 1) {
    result = fibRecInner(n - 1) + fibRecInner(n - 2);
  }
  return result;
}

fn fibRec() -> Int {
  return fibRecInner(30);
}

fn stringScan() -> Int {
  let text = "alpha-beta-gamma-delta-skepa-language-benchmark";
  let acc = 0;
  let i = 0;
  while (i < 120000) {
    if (str.contains(text, "gamma")) {
      acc = acc + str.len(text) + (i % 97);
    }
    if (str.indexOf(text, "alpha") == 0) {
      acc = acc + 11;
    }
    if (str.indexOf(text, "benchmark") >= 0) {
      acc = acc + 17;
    }
    acc = acc % 1000000007;
    i = i + 1;
  }
  return acc;
}

fn optionResult() -> Int {
  let acc = 0;
  let i = 1;
  while (i <= 80000) {
    let maybe: Option[Int] = Some(i * 7 + 5);
    if (i % 11 == 0) {
      maybe = None();
    }
    match (maybe) {
      None => {
        acc = acc + 3;
      }
      Some(value) => {
        let res: Result[Int, Int] = Ok(value);
        if (value % 17 == 0) {
          res = Err(19);
        }
        match (res) {
          Ok(v) => {
            acc = (acc + v) % 1000000007;
          }
          Err(e) => {
            acc = acc + e;
          }
        }
      }
    }
    i = i + 1;
  }
  return acc % 1000000007;
}

fn main() -> Int {
  let name = option.unwrapSome(os.arg(1));
  let answer = -1;
  if (name == "arith_loop") {
    answer = arithLoop();
  } else if (name == "bitmix") {
    answer = bitmix();
  } else if (name == "nested_loops") {
    answer = nestedLoops();
  } else if (name == "fib_iter") {
    answer = fibIter();
  } else if (name == "gcd_chain") {
    answer = gcdChain();
  } else if (name == "prime_count") {
    answer = primeCount();
  } else if (name == "collatz") {
    answer = collatz();
  } else if (name == "branch_mix") {
    answer = branchMix();
  } else if (name == "function_calls") {
    answer = functionCalls();
  } else if (name == "vec_push_sum") {
    answer = vecPushSum();
  } else if (name == "matrix_walk") {
    answer = matrixWalk();
  } else if (name == "fib_rec") {
    answer = fibRec();
  } else if (name == "string_scan") {
    answer = stringScan();
  } else if (name == "option_result") {
    answer = optionResult();
  }
  io.printInt(answer);
  io.println("");
  let code = 0;
  if (answer < 0) {
    code = 2;
  }
  return code;
}
'@
}

New-CleanDir $WorkRoot
Ensure-Dir $SrcRoot
Ensure-Dir $BinRoot
Ensure-Dir $ResultDir

Write-TextFile (Join-Path $SrcRoot "bench.sk") (Common-Skepa)
Write-TextFile (Join-Path $SrcRoot "bench.py") (Common-Python)
Write-TextFile (Join-Path $SrcRoot "bench.c") (Common-C)
Write-TextFile (Join-Path $SrcRoot "bench.cpp") (Common-Cpp)
Write-TextFile (Join-Path $SrcRoot "Bench.java") (Common-Java)
Write-TextFile (Join-Path $SrcRoot "bench.rs") (Common-Rust)
Write-TextFile (Join-Path $SrcRoot "bench.js") (Common-Node)
$DataFile = Join-Path $WorkRoot "data.txt"
Write-TextFile $DataFile (New-DataText)

$BuildRows = @()
$RunRows = @()
$Programs = @{}
$SkepaSupportedCases = @(
    "arith_loop",
    "bitmix",
    "nested_loops",
    "fib_iter",
    "gcd_chain",
    "prime_count",
    "collatz",
    "branch_mix",
    "function_calls",
    "vec_push_sum",
    "matrix_walk",
    "fib_rec",
    "string_scan"
)

function Add-BuildRow($Language, $Status, $Ms, $Command, $Message) {
    $script:BuildRows += [pscustomobject]@{
        language = $Language
        status = $Status
        build_ms = if ($null -eq $Ms) { "" } else { [math]::Round($Ms, 3) }
        command = $Command
        message = $Message
    }
}

function Register-Program {
    param(
        [string]$Language,
        [string]$File,
        [string[]]$ArgsPrefix,
        [string]$WorkDir,
        [string[]]$SupportedCases = $Cases
    )
    $script:Programs[$Language] = [pscustomobject]@{
        File = $File
        ArgsPrefix = @($ArgsPrefix)
        WorkDir = $WorkDir
        SupportedCases = @($SupportedCases)
    }
}

foreach ($Language in $Languages) {
    switch ($Language) {
        "skepa" {
            $Tool = Resolve-Skepac
            if (!$Tool) {
                Add-BuildRow "skepa" "skipped" $null "skepac build-native" "skepac not found; build it with cargo build -p skepac or pass -Skepac"
                break
            }
            $Out = Join-Path $BinRoot "skepa_bench.exe"
            $Result = Invoke-TimedCommand -File $Tool -CommandArgs @("build-native", (Join-Path $SrcRoot "bench.sk"), $Out) -WorkingDirectory $Root
            if ($Result.ExitCode -eq 0) {
                Add-BuildRow "skepa" "ok" $Result.Ms "$Tool build-native" ""
                Register-Program -Language "skepa" -File $Out -ArgsPrefix @() -WorkDir $BinRoot -SupportedCases $SkepaSupportedCases
            } else {
                Add-BuildRow "skepa" "failed" $Result.Ms "$Tool build-native" ($Result.Stdout + " " + $Result.Stderr)
            }
        }
        "python" {
            $Tool = Find-Tool @("python", "py")
            if (!$Tool) {
                Add-BuildRow "python" "skipped" $null "python" "python not found"
                break
            }
            Add-BuildRow "python" "ok" 0 $Tool "interpreted"
            Register-Program -Language "python" -File $Tool -ArgsPrefix @((Join-Path $SrcRoot "bench.py")) -WorkDir $Root
        }
        "node" {
            $Tool = Find-Tool @("node")
            if (!$Tool) {
                Add-BuildRow "node" "skipped" $null "node" "node not found"
                break
            }
            Add-BuildRow "node" "ok" 0 $Tool "interpreted/JIT"
            Register-Program -Language "node" -File $Tool -ArgsPrefix @((Join-Path $SrcRoot "bench.js")) -WorkDir $Root
        }
        "c" {
            $Tool = Find-Tool @("gcc", "clang")
            if (!$Tool) {
                Add-BuildRow "c" "skipped" $null "gcc/clang" "C compiler not found"
                break
            }
            $Out = Join-Path $BinRoot "c_bench.exe"
            $Result = Invoke-TimedCommand -File $Tool -CommandArgs @("-O2", "-std=c11", (Join-Path $SrcRoot "bench.c"), "-o", $Out) -WorkingDirectory $Root
            if ($Result.ExitCode -eq 0) {
                Add-BuildRow "c" "ok" $Result.Ms "$Tool -O2" ""
                Register-Program -Language "c" -File $Out -ArgsPrefix @() -WorkDir $BinRoot
            } else {
                Add-BuildRow "c" "failed" $Result.Ms "$Tool -O2" ($Result.Stdout + " " + $Result.Stderr)
            }
        }
        "cpp" {
            $Tool = Find-Tool @("g++", "clang++")
            if (!$Tool) {
                Add-BuildRow "cpp" "skipped" $null "g++/clang++" "C++ compiler not found"
                break
            }
            $Out = Join-Path $BinRoot "cpp_bench.exe"
            $Result = Invoke-TimedCommand -File $Tool -CommandArgs @("-O2", "-std=c++17", (Join-Path $SrcRoot "bench.cpp"), "-o", $Out) -WorkingDirectory $Root
            if ($Result.ExitCode -eq 0) {
                Add-BuildRow "cpp" "ok" $Result.Ms "$Tool -O2" ""
                Register-Program -Language "cpp" -File $Out -ArgsPrefix @() -WorkDir $BinRoot
            } else {
                Add-BuildRow "cpp" "failed" $Result.Ms "$Tool -O2" ($Result.Stdout + " " + $Result.Stderr)
            }
        }
        "java" {
            $Javac = Find-Tool @("javac")
            $Java = Find-Tool @("java")
            if (!$Javac -or !$Java) {
                Add-BuildRow "java" "skipped" $null "javac/java" "Java toolchain not found"
                break
            }
            $JavaOut = Join-Path $BinRoot "java"
            Ensure-Dir $JavaOut
            $Result = Invoke-TimedCommand -File $Javac -CommandArgs @("-d", $JavaOut, (Join-Path $SrcRoot "Bench.java")) -WorkingDirectory $Root
            if ($Result.ExitCode -eq 0) {
                Add-BuildRow "java" "ok" $Result.Ms "$Javac" ""
                Register-Program -Language "java" -File $Java -ArgsPrefix @("-cp", $JavaOut, "Bench") -WorkDir $Root
            } else {
                Add-BuildRow "java" "failed" $Result.Ms "$Javac" ($Result.Stdout + " " + $Result.Stderr)
            }
        }
        "rust" {
            $Tool = Find-Tool @("rustc")
            if (!$Tool) {
                Add-BuildRow "rust" "skipped" $null "rustc" "rustc not found"
                break
            }
            $Out = Join-Path $BinRoot "rust_bench.exe"
            $Result = Invoke-TimedCommand -File $Tool -CommandArgs @("-O", (Join-Path $SrcRoot "bench.rs"), "-o", $Out) -WorkingDirectory $Root
            if ($Result.ExitCode -eq 0) {
                Add-BuildRow "rust" "ok" $Result.Ms "$Tool -O" ""
                Register-Program -Language "rust" -File $Out -ArgsPrefix @() -WorkDir $BinRoot
            } else {
                Add-BuildRow "rust" "failed" $Result.Ms "$Tool -O" ($Result.Stdout + " " + $Result.Stderr)
            }
        }
        default {
            Add-BuildRow $Language "skipped" $null $Language "unknown language"
        }
    }
}

$ExpectedByCase = @{}

foreach ($Language in $Languages) {
    if (!$Programs.ContainsKey($Language)) {
        continue
    }
    $Program = $Programs[$Language]
    foreach ($Case in $Cases) {
        if (@($Program.SupportedCases) -notcontains $Case) {
            $RunRows += [pscustomobject]@{
                language = $Language
                case = $Case
                status = "unsupported"
                checksum = ""
                checksum_match = $false
                runs = 0
                min_ms = ""
                median_ms = ""
                avg_ms = ""
                max_ms = ""
                message = "not supported by this language/backend template"
            }
            continue
        }
        for ($i = 0; $i -lt $Warmups; $i++) {
            [void](Invoke-TimedCommand -File $Program.File -CommandArgs (@($Program.ArgsPrefix) + @($Case, $DataFile)) -WorkingDirectory $Program.WorkDir)
        }

        $Times = @()
        $Checksum = ""
        $Status = "ok"
        $Message = ""
        for ($i = 0; $i -lt $Runs; $i++) {
            $Result = Invoke-TimedCommand -File $Program.File -CommandArgs (@($Program.ArgsPrefix) + @($Case, $DataFile)) -WorkingDirectory $Program.WorkDir
            if ($Result.ExitCode -ne 0) {
                $Status = "failed"
                $Message = $Result.Stdout + " " + $Result.Stderr
                break
            }
            $Out = ($Result.Stdout -split "\r?\n" | Select-Object -First 1).Trim()
            if ($Out -eq "") {
                $Status = "empty-output"
                $Message = "benchmark exited successfully but printed no checksum"
                break
            }
            if ($Checksum -eq "") {
                $Checksum = $Out
            } elseif ($Checksum -ne $Out) {
                $Status = "unstable-output"
                $Message = "saw both $Checksum and $Out"
                break
            }
            $Times += $Result.Ms
        }

        if ($Status -eq "ok") {
            if (!$ExpectedByCase.ContainsKey($Case)) {
                $ExpectedByCase[$Case] = $Checksum
            }
            if ($ExpectedByCase[$Case] -ne $Checksum) {
                $Status = "checksum-mismatch"
                $Message = "expected $($ExpectedByCase[$Case]), got $Checksum"
            }
        }

        $RunRows += [pscustomobject]@{
            language = $Language
            case = $Case
            status = $Status
            checksum = $Checksum
            checksum_match = ($Status -eq "ok")
            runs = $Times.Count
            min_ms = if ($Times.Count) { [math]::Round(($Times | Measure-Object -Minimum).Minimum, 3) } else { "" }
            median_ms = if ($Times.Count) { [math]::Round((Median $Times), 3) } else { "" }
            avg_ms = if ($Times.Count) { [math]::Round((Average $Times), 3) } else { "" }
            max_ms = if ($Times.Count) { [math]::Round(($Times | Measure-Object -Maximum).Maximum, 3) } else { "" }
            message = $Message
        }
    }
}

function GeoMean($Values) {
    $Filtered = @($Values | Where-Object { $null -ne $_ -and [double]$_ -gt 0 })
    if ($Filtered.Count -eq 0) {
        return $null
    }
    $TotalLog = 0.0
    foreach ($Value in $Filtered) {
        $TotalLog += [math]::Log([double]$Value)
    }
    return [math]::Exp($TotalLog / $Filtered.Count)
}

$BestRuntimeByCase = @{}
foreach ($Case in $Cases) {
    $OkRows = @($RunRows | Where-Object { $_.case -eq $Case -and $_.status -eq "ok" -and $_.median_ms -ne "" })
    if ($OkRows.Count -gt 0) {
        $BestRuntimeByCase[$Case] = ($OkRows | ForEach-Object { [double]$_.median_ms } | Measure-Object -Minimum).Minimum
    }
}

$PositiveBuilds = @($BuildRows | Where-Object { $_.status -eq "ok" -and $_.build_ms -ne "" -and [double]$_.build_ms -gt 0 } | ForEach-Object { [double]$_.build_ms })
$FastestBuild = if ($PositiveBuilds.Count -gt 0) { ($PositiveBuilds | Measure-Object -Minimum).Minimum } else { $null }
$ScoreRows = @()
foreach ($Language in $Languages) {
    $Rows = @($RunRows | Where-Object { $_.language -eq $Language -and $_.status -eq "ok" -and $_.median_ms -ne "" })
    if ($Rows.Count -eq 0) {
        continue
    }
    $RuntimeParts = @()
    foreach ($Row in $Rows) {
        if ($BestRuntimeByCase.ContainsKey($Row.case)) {
            $RuntimeParts += ([double]$BestRuntimeByCase[$Row.case] / [double]$Row.median_ms) * 100.0
        }
    }
    $RuntimeIndex = GeoMean $RuntimeParts
    $BuildRow = @($BuildRows | Where-Object { $_.language -eq $Language } | Select-Object -First 1)
    $BuildMs = if ($BuildRow.Count -gt 0 -and $BuildRow[0].build_ms -ne "") { [double]$BuildRow[0].build_ms } else { 0.0 }
    $BuildIndex = 100.0
    if ($BuildMs -gt 0 -and $null -ne $FastestBuild -and $FastestBuild -gt 0) {
        $BuildIndex = ([double]$FastestBuild / $BuildMs) * 100.0
    }
    $Composite = ($RuntimeIndex * 0.85) + ($BuildIndex * 0.15)
    $ScoreRows += [pscustomobject]@{
        language = $Language
        ok_cases = $Rows.Count
        runtime_index = [math]::Round($RuntimeIndex, 3)
        build_index = [math]::Round($BuildIndex, 3)
        composite_score = [math]::Round($Composite, 3)
        build_ms = if ($BuildMs -gt 0) { [math]::Round($BuildMs, 3) } else { 0 }
    }
}

$SkepaRows = @($RunRows | Where-Object { $_.language -eq "skepa" -and $_.status -eq "ok" -and $_.median_ms -ne "" })
$SkepaBuildRow = @($BuildRows | Where-Object { $_.language -eq "skepa" } | Select-Object -First 1)
$SkepaBuildMs = if ($SkepaBuildRow.Count -gt 0 -and $SkepaBuildRow[0].build_ms -ne "") { [double]$SkepaBuildRow[0].build_ms } else { 0.0 }
$HeadToHeadRows = @()
if ($SkepaRows.Count -gt 0) {
    foreach ($Language in $Languages) {
        if ($Language -eq "skepa") {
            continue
        }
        $Factors = @()
        foreach ($SkepaRow in $SkepaRows) {
            $OtherRow = @($RunRows | Where-Object { $_.language -eq $Language -and $_.case -eq $SkepaRow.case -and $_.status -eq "ok" -and $_.median_ms -ne "" } | Select-Object -First 1)
            if ($OtherRow.Count -gt 0) {
                $Factors += ([double]$OtherRow[0].median_ms / [double]$SkepaRow.median_ms)
            }
        }
        if ($Factors.Count -eq 0) {
            continue
        }
        $RuntimeFactor = GeoMean $Factors
        $OtherBuildRow = @($BuildRows | Where-Object { $_.language -eq $Language } | Select-Object -First 1)
        $OtherBuildMs = if ($OtherBuildRow.Count -gt 0 -and $OtherBuildRow[0].build_ms -ne "") { [double]$OtherBuildRow[0].build_ms } else { 0.0 }
        $BuildFactor = 1.0
        if ($SkepaBuildMs -gt 0 -and $OtherBuildMs -gt 0) {
            $BuildFactor = $OtherBuildMs / $SkepaBuildMs
        }
        $PowerIndex = (($RuntimeFactor * 0.85) + ($BuildFactor * 0.15)) * 100.0
        $HeadToHeadRows += [pscustomobject]@{
            vs_language = $Language
            common_cases = $Factors.Count
            skepa_runtime_factor = [math]::Round($RuntimeFactor, 3)
            skepa_build_factor = [math]::Round($BuildFactor, 3)
            skepa_power_index = [math]::Round($PowerIndex, 3)
            interpretation = if ($PowerIndex -ge 100.0) { "skepa ahead by this model" } else { "skepa behind by this model" }
        }
    }
}

$BuildRows | Export-Csv -NoTypeInformation -Path (Join-Path $ResultDir "builds.csv")
$RunRows | Export-Csv -NoTypeInformation -Path (Join-Path $ResultDir "runs.csv")
$ScoreRows | Sort-Object composite_score -Descending | Export-Csv -NoTypeInformation -Path (Join-Path $ResultDir "scores.csv")
$HeadToHeadRows | Sort-Object skepa_power_index -Descending | Export-Csv -NoTypeInformation -Path (Join-Path $ResultDir "skepa-head-to-head.csv")
@{
    generated_at = (Get-Date).ToString("o")
    runs = $Runs
    warmups = $Warmups
    cases = $Cases
    builds = $BuildRows
    results = $RunRows
    scores = $ScoreRows
    skepa_head_to_head = $HeadToHeadRows
} | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $ResultDir "results.json") -Encoding UTF8

$Summary = New-Object System.Collections.Generic.List[string]
$Summary.Add("# Cross-Language Benchmark Results")
$Summary.Add("")
$Summary.Add("- generated: $(Get-Date -Format o)")
$Summary.Add("- runs per case: $Runs")
$Summary.Add("- warmups per case: $Warmups")
$Summary.Add("- scoring model: runtime index is geometric mean of best-runtime-per-case normalized scores; build index is normalized to the fastest nonzero build; composite score = 85% runtime index + 15% build index")
$Summary.Add("- Skepa head-to-head: 100 means parity; above 100 means Skepa is ahead by the model; runtime factor above 1.0 means Skepa ran faster")
$Summary.Add("")
$Summary.Add("## Build Summary")
$Summary.Add("")
$Summary.Add("| language | status | build ms | command | message |")
$Summary.Add("| --- | --- | ---: | --- | --- |")
foreach ($Row in $BuildRows) {
    $CommandText = ([string]$Row.command) -replace '\|','/'
    $MessageText = ([string]$Row.message) -replace '\|','/'
    $Summary.Add("| $($Row.language) | $($Row.status) | $($Row.build_ms) | $CommandText | $MessageText |")
}
$Summary.Add("")
$Summary.Add("## Scorecard")
$Summary.Add("")
$Summary.Add("| language | ok cases | runtime index | build index | composite score | build ms |")
$Summary.Add("| --- | ---: | ---: | ---: | ---: | ---: |")
foreach ($Row in ($ScoreRows | Sort-Object composite_score -Descending)) {
    $Summary.Add("| $($Row.language) | $($Row.ok_cases) | $($Row.runtime_index) | $($Row.build_index) | $($Row.composite_score) | $($Row.build_ms) |")
}
$Summary.Add("")
$Summary.Add("## Skepa Head-To-Head")
$Summary.Add("")
$Summary.Add("| vs language | common cases | runtime factor | build factor | power index | interpretation |")
$Summary.Add("| --- | ---: | ---: | ---: | ---: | --- |")
foreach ($Row in ($HeadToHeadRows | Sort-Object skepa_power_index -Descending)) {
    $Summary.Add("| $($Row.vs_language) | $($Row.common_cases) | $($Row.skepa_runtime_factor) | $($Row.skepa_build_factor) | $($Row.skepa_power_index) | $($Row.interpretation) |")
}
$Summary.Add("")
$Summary.Add("## Runtime Summary")
$Summary.Add("")
$Summary.Add("| language | case | status | checksum | median ms | min ms | max ms | avg ms |")
$Summary.Add("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |")
foreach ($Row in ($RunRows | Sort-Object case, language)) {
    $Summary.Add("| $($Row.language) | $($Row.case) | $($Row.status) | $($Row.checksum) | $($Row.median_ms) | $($Row.min_ms) | $($Row.max_ms) | $($Row.avg_ms) |")
}
$Summary | Set-Content -Path (Join-Path $ResultDir "summary.md") -Encoding UTF8

Write-Host ""
Write-Host "Build summary"
$BuildRows | Format-Table language, status, build_ms, command -AutoSize

Write-Host ""
Write-Host "Runtime summary"
$RunRows | Sort-Object case, language | Format-Table language, case, status, checksum, median_ms, min_ms, max_ms -AutoSize

Write-Host ""
Write-Host "Scorecard"
$ScoreRows | Sort-Object composite_score -Descending | Format-Table language, ok_cases, runtime_index, build_index, composite_score -AutoSize

Write-Host ""
Write-Host "Skepa head-to-head"
$HeadToHeadRows | Sort-Object skepa_power_index -Descending | Format-Table vs_language, common_cases, skepa_runtime_factor, skepa_build_factor, skepa_power_index, interpretation -AutoSize

Write-Host ""
Write-Host "Results written to: $ResultDir"
