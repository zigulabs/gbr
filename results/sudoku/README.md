# 스도쿠 1만 판 결과 · 정답 확인 증명서 · 검사기

[English below](#sudoku-10000-puzzle-result--answer-certificates--checker)

공개 데이터셋의 시험 묶음에서 결과를 보기 전에 정한 규칙으로 뽑은 1만 판을 추론 엔진으로 풀었습니다. 판마다 엔진의 답이 그 판의 유일한 해임을 보이는 정답 확인 증명서를 붙였습니다. 증명서는 판과 답만 받는 공개 증명기로 지었고, 엔진이 어떻게 풀었는지는 담지 않습니다. 누구나 데이터셋 원본과 이 증명서만으로 답을 다시 확인할 수 있습니다.

## 데이터와 뽑은 판

| 항목 | 값 |
|---|---|
| 데이터셋 | Hugging Face `sapientinc/sudoku-extreme`의 `test.csv`(422,786판 · 파일 sha256 `a2fd52aea23d331d5b4ee723c856236e838a9fb9a70e66f4e0e0cf26c338c6a8`) |
| 판 이름 | `test.csv`의 데이터 줄 번호(머리 줄 다음 줄이 1) |
| 뽑는 규칙 | 판 번호 n마다 열쇠 = sha256(`GBR_SUDOKU_EXTREME_TEST_PUBLIC_WHOLE_V1` · 0x00 · n의 10진 글자), 열쇠 오름차순(같으면 n 차례)으로 앞 10,000판 · 결과를 보기 전에 고정 |
| 개발 때 본 판 | 1만 판 가운데 2,621판은 개발 중에 쓴 적이 있습니다(난이도 100 이상 115판은 모두 여기에 듭니다). 이 판들을 뺀 셈도 따로 밝힙니다. |

## 결과

| 셈 | 판 | 풀고 확인됨 |
|---|---|---|
| 전체 | 10,000 | 10,000 |
| 개발 때 안 본 판만 | 7,379 | 7,379 |
| 난이도 0 | 1,410 | 1,410 |
| 난이도 1–9 | 2,386 | 2,386 |
| 난이도 10–49 | 5,102 | 5,102 |
| 난이도 50–99 | 987 | 987 |
| 난이도 100 이상 | 115 | 115 |

- 「확인됨」 = 아래 검사기가 그 판의 증명서를 받아들이고, 증명서가 정하는 유일한 해가 데이터셋의 정답과 같음. 증명서는 엔진의 답을 넣어 지었습니다. 결과표의 `answer_sha256`은 엔진이 낸 81자 답의 sha256이고, 1만 판 모두 데이터셋 정답의 sha256과 같습니다. 바깥에서 확인할 수 있는 것은 「데이터셋의 정답이 그 판의 유일한 해다」와 「우리가 밝힌 엔진의 답이 그 정답과 같다」까지입니다. 엔진이 그 답을 냈다는 근거(풀이 과정 증명서)는 다음 줄처럼 안에서 확인했습니다.
- 판마다 한 번 풀었고, 추측도 되돌림도 하지 않았습니다. 여기서 「추측 없음」은 어떤 후보도 맞다고 가정한 채 풀이를 이어 가지 않는다는 뜻입니다. 이 성질은 안에서 확인했습니다. 풀이 과정을 따라 만든 증명서 1만 개를 같은 검사기와 drat-trim으로 모두 확인했고, 그 증명서는 풀이 방법을 드러내므로 이 묶음에 싣지 않습니다. 저장된 기억은 읽지 않았습니다.
- 추론 엔진을 직접 돌린 결과이며, 시스템 전체 경로로 낸 점수는 아닙니다. 앞 100판은 시스템 전체 경로로 낸 답과 판마다 같았습니다.
- 확인은 두 가지입니다. 함께 실은 검사기가 정답 확인 증명서 1만 개를 모두 받아들였고(받아들임 10,000 · 거절 0), 판마다 데이터셋 정답과 맞대어 보니 틀린 판은 없었습니다. 공개 증명기는 이 검사기의 CNF와 단위 전파 코드를 불러 증명서를 짓고 스스로 확인하므로, 이 확인은 증명기와 코드를 나눠 씁니다. 그래서 우리가 짓지 않은 바깥 검사기 drat-trim으로도 1만 개를 모두 확인했습니다(VERIFIED 10,000 · 아래 「검사기와 증명기」).
- 난이도 띠마다 1만 판과 시험 묶음 전체의 분포는 `sudoku_10k_distribution.tsv`에 있습니다.
- 같은 엔진으로 이 데이터셋 밖의 판 726개도 풀었습니다(아래 「데이터셋 밖 판」).

## 스도쿠 규칙은 예시에서 셉니다 (표준 9×9)

엔진은 판 규칙(어느 칸끼리 같은 숫자가 올 수 없는지)을 판의 크기에서 짓지 않고, 공개 데이터셋 훈련 묶음(`train.csv`)의 정답 예시에서 셉니다.

- `train.csv` 앞에서부터 정답 81개를 세면, 센 규칙이 표준 스도쿠 규칙(행 · 열 · 상자)과 같아집니다.
- 센 규칙으로 1만 판을 다시 풀면 엔진의 출력이 위 결과와 한 바이트도 다르지 않습니다. 답이 같으므로 증명서도 같습니다.
- 공개 길은 센 규칙이 표준 규칙과 같은지 맞대어, 다르면 풀지 않고 멈춥니다.

이 절의 말은 표준 9×9에만 해당합니다. 변형 스도쿠는 이 묶음에 없습니다.

## 실험 환경

| 항목 | 값 |
|---|---|
| 장비 | 데스크톱급 PC 한 대 · Intel Core i5-13500(14코어 · 20스레드) · 메모리 64 GB · NVMe SSD |
| GPU | 쓰지 않음(CPU만) |
| 운영체제 | Ubuntu 24.04 LTS |
| 풀이 | 판마다 한 번 · 동시에 10판씩 · 1만 판 1시간 56분(6,939초) · 풀이 프로세스 하나 최대 약 2.1 GB |
| 증명서 짓기 | 공개 증명기(Rust 1.96) · 1만 판을 16묶음으로 나눠 동시에 159초 · 합친 CPU 시간 2,129초 |
| 검사 | 공개 검사기(Rust 1.96): 1만 판을 프로세스 하나로 10.6초 · 최대 메모리 약 90 MB · drat-trim(커밋 `2e3b2dc0`, gcc 13.3): 1만 판을 16개씩 동시에 88초 |
| 데이터 | `test.csv` sha256 `a2fd52aea23d331d5b4ee723c856236e838a9fb9a70e66f4e0e0cf26c338c6a8` |

## 파일

| 파일 | 담는 것 |
|---|---|
| `sudoku_10k_results.tsv` | 판마다 한 줄: `line` · `line_sha256`(그 줄 바이트의 sha256) · `rating` · `seen_in_development` · `outcome` · `answer_sha256`(엔진이 낸 81자 답의 sha256) · `proof_lemmas`(그 판 증명서의 보조 절 수) · `correct` · 문제 글과 격자는 싣지 않음 |
| `sudoku_10k_counts.tsv` | 셈 꼴 셋: 전체 · 안 본 판만 · 난이도 띠마다(본 판 몫 함께) |
| `sudoku_10k_distribution.tsv` | 난이도 띠마다 시험 묶음 전체 판 수 · 몫, 1만 판 판 수 · 몫 · 본 판 · 안 본 판 · 풀고 확인된 판 |
| `sudoku_10k_drat_trim.txt` | 바깥 검사기 drat-trim 확인: 증명서 묶음 sha256 · 핀(원천 커밋 · 파일 sha256 · 짓기 명령) · 판마다 명령 · 셈 |
| `certificates/sudoku_10k_certificates.tar.gz` | 판마다 정답 확인 증명서 `proofs/<line>.drup` 10,000개(풀면 4,330,726바이트) |
| `verify/sudoku-rup-check/` | 검사기 소스(Rust · 표준 라이브러리만) |
| `verify/sudoku-rup-prover/` | 공개 증명기 소스(Rust · 표준 라이브러리와 위 검사기 크레이트만) |
| `certificate_samples/` | 증명서 셋(103 · 1581 · 5451, 풀어 보지 않고 읽어 볼 수 있게 · 103은 보조 절이 없어 빈 파일) |
| `outside_dataset_726/results.tsv` | 데이터셋 밖 726판: 판마다 `line`(증명서 이름) · `set` · `source` · `source_line` · `rating` · `puzzle_sha256` · `answer_sha256`(엔진이 낸 81자 답의 sha256) · 데이터셋 겹침(`dataset_same_text` · `dataset_same_up_to_symmetry` · `dataset_match`) · `solved` · `checker` · `drat_trim` · 판 글과 격자는 싣지 않음 |
| `outside_dataset_726/certificates.tar.gz` | 726판의 정답 확인 증명서 `proofs/<line>.drup` |
| `outside_dataset_726/build_boards.py` | 원천 파일에서 판 글을 꺼내(sha256 맞댐) 검사기 입력을 만듦 |
| `outside_dataset_726/sources.txt` | 원천 넷의 주소 · 바이트 · sha256, 고르는 규칙, 데이터셋 겹침 셈 |
| `SHA256SUMS` | 위 파일의 sha256 |

문제 글과 격자를 싣지 않는 까닭: 데이터셋 카드에 라이선스 표기가 없습니다. `line`과 `line_sha256`으로 데이터셋의 원래 줄과 맞대어 볼 수 있습니다.

## 증명서의 꼴

- 판마다 표준 스도쿠 CNF를 씁니다. 변수는 `81·행 + 9·열 + 숫자`(행 · 열은 0–8, 숫자는 1–9 · 변수 1 ~ 729)이고, 칸 · 행 · 열 · 상자마다 「적어도 하나」 · 「둘은 아님」 절을 두며, 주어진 숫자마다 단위 절을 더합니다.
- 증명서는 이 CNF에 더하는 보조 절의 목록입니다. 한 줄에 절 하나를 DIMACS 꼴로 쓰고 `0`으로 끝냅니다. 더하기만 있습니다(DRUP).
- 보조 절은 차례대로 앞의 절들에 대해 단위 전파로 확인됩니다(RUP). 모두 받아들여진 뒤 단위 전파가 81칸을 모두 정하면, 그 격자가 유일한 해입니다.
- 증명서는 공개 증명기가 판과 답 두 글만 받아 고정된 공개 규칙으로 짓습니다(규칙은 `verify/sudoku-rup-prover/README.md`). 같은 판과 답이면 늘 같은 바이트가 나옵니다. 풀이 엔진의 코드 · 차례 · 표는 쓰지 않습니다.
- 1만 개의 보조 절은 모두 500,921개입니다(단위 절 267,431 · 리터럴이 둘인 절 233,490 · 더 긴 절 0). 1,068판은 단위 전파만으로 81칸이 정해져 증명서가 비어 있습니다. 판마다 보조 절 수: 0개 1,068판 · 1–9개 1,487판 · 10–99개 6,303판 · 100–999개 1,142판.

## 검사기와 증명기

```
curl -L -o test.csv https://huggingface.co/datasets/sapientinc/sudoku-extreme/resolve/main/test.csv
echo "a2fd52aea23d331d5b4ee723c856236e838a9fb9a70e66f4e0e0cf26c338c6a8  test.csv" | sha256sum -c
cargo build --release --manifest-path verify/sudoku-rup-check/Cargo.toml
mkdir certs && tar -xzf certificates/sudoku_10k_certificates.tar.gz -C certs
verify/sudoku-rup-check/target/release/sudoku-rup-check dataset test.csv certs/proofs
```

- 앞 두 줄은 데이터셋 원본을 내려받아 sha256을 맞댑니다(`test.csv: OK`). 아래 명령은 모두 이 묶음 폴더에서 돌립니다.
- 판마다 탭으로 나눈 한 줄 `<line>` · `VERIFIED` · `matches_answer lemmas=<n>`(또는 `OPEN` · `FAILED <까닭>`)을 찍고, 끝에 `TOTAL boards=… verified=… open=… failed=…`을 찍습니다. 끝 줄이 `TOTAL boards=10000 verified=10000 open=0 failed=0`이면 1만 판이 모두 확인된 것입니다.
- 바깥 검사기 drat-trim(공개 원천 커밋 `2e3b2dc0`)으로 1만 판 모두 확인했습니다: VERIFIED 10,000 · NOT VERIFIED 0. 판마다 `sudoku-rup-check dimacs <판 81자> <격자 81자>`(격자를 막는 절을 더한 CNF)와 `sudoku-rup-check drat <증명서>`(끝에 빈 절을 붙인 DRAT 글)를 `drat-trim`에 넣었고, VERIFIED는 「그 격자 말고는 해가 없음」을 뜻합니다. 핀 · 명령 · 셈은 `sudoku_10k_drat_trim.txt`에 있습니다.

증명서를 다시 지어 보려면(판과 답만 넣습니다):

```
cargo build --release --manifest-path verify/sudoku-rup-prover/Cargo.toml
tail -n +2 sudoku_10k_results.tsv | cut -f1 > lines.txt
awk -F, 'NR == FNR {want[$1]; next} FNR > 1 {n = FNR - 1; if (n in want) print n "\t" $2 "\t" $3}' lines.txt test.csv > boards.tsv
mkdir proofs2
verify/sudoku-rup-prover/target/release/sudoku-rup-prover batch boards.tsv proofs2
diff -r certs/proofs proofs2
```

- `boards.tsv`는 1만 판마다 `<line>` · `question` · `answer`(`test.csv` 그 줄의 두 칸)를 탭으로 나눈 한 줄입니다. 엔진의 답은 1만 판 모두 데이터셋의 정답과 같으므로(`answer_sha256`로 확인), 데이터셋의 정답을 넣으면 엔진의 답을 넣은 것과 같은 입력입니다.
- 끝 줄이 `TOTAL boards=10000 proved=10000 failed=0`이고 `diff -r`이 아무것도 찍지 않으면, 실린 증명서와 바이트까지 같은 증명서가 다시 지어진 것입니다. 한 프로세스로는 오래 걸리므로(합친 CPU 시간 약 35분) `split`으로 `boards.tsv`를 나눠 여러 개를 동시에 돌려도 됩니다. 같은 파일이 나옵니다.
- `answer_sha256`은 `printf '%s' '<answer 칸>' | sha256sum`의 값입니다.

## 데이터셋 밖 판

같은 엔진 · 같은 조건(판마다 한 번)으로, 공개 목록 넷에서 결과를 보기 전에 정한 규칙으로 고른 표준 9×9 726판을 풀었습니다(원천과 규칙은 `outside_dataset_726/sources.txt`).

| 묶음 | 원천 | 판 | 데이터셋과 같은 판 | 데이터셋 밖 | 풀고 확인됨 |
|---|---|---|---|---|---|
| 이름난 판 | 널리 알려진 난제 넷 + attractivechaos/plb `sudoku.txt` 20줄 | 23 | 22 | 1 | 23 |
| Norvig | norvig.com `top95.txt` · `hardest.txt` | 104 | 93 | 11 | 104 |
| 퍼즐 은행 | Sudoku Exchange `diabolical.txt`(퍼블릭 도메인) 앞 500줄 | 500 | 0 | 500 | 500 |
| 퍼즐 은행 최고 등급 | 같은 파일에서 등급이 가장 높은 100줄(앞 묶음과 겹친 한 판 뺌) | 99 | 0 | 99 | 99 |
| 합 | | 726 | 115 | 611 | 726 |

- 「데이터셋과 같은 판」 = 데이터셋(`train.csv` · `test.csv`)의 어떤 판과 글자가 같거나, 스도쿠 대칭(전치 · 띠와 더미의 차례 · 띠 안 행과 더미 안 열의 차례 · 숫자 바꿈)으로 옮기면 같아지는 판입니다. 글자가 같은 판은 0이었고, 115판이 대칭으로 같았습니다. 이름난 판은 AI Escargot 하나 말고 모두 데이터셋 안에 있었습니다.
- 데이터셋 밖 611판은 611/611 풀고 확인됐습니다. 바깥 출처의 판이므로 「누구도 본 적 없는 판」이 아니라 「이 데이터셋 밖의 판」입니다.
- 「확인됨」 = 엔진의 답으로 공개 증명기가 지은 증명서를 공개 검사기가 받아들이고(그 답이 유일한 해), drat-trim도 VERIFIED.
- 바깥에서 확인할 수 있는 것은 「판마다 증명서가 유일한 해를 정하고, 그 해의 sha256이 우리가 밝힌 엔진 답의 sha256(`answer_sha256`)과 같다」까지입니다. 엔진이 그 답을 냈다는 근거(풀이 과정 증명서)는 1만 판과 같이 안에서 확인했습니다.
- 판 글은 이 절의 표와 파일에 싣지 않습니다(원천마다 사용 조건이 다름). 예외로, 공개 검사기 · 증명기의 시험 파일(`verify/sudoku-rup-check/tests/cells.rs` · `verify/sudoku-rup-prover/tests/cells.rs`)에는 널리 알려진 판 몇 개가 시험 자료로 들어 있고, 726판 가운데 넷(AI Escargot · Inkala 2012 · Easter Monster · `top95.txt` 첫 줄)이 거기에 있습니다(Inkala 2012는 정답 격자까지). `build_boards.py`는 네 원천 파일 어디에도 없는 Inkala 2012를 증명기 시험 파일에서 꺼냅니다.
- `outside_dataset_726/results.tsv`에 판마다 원천 파일과 줄, 판 글의 sha256, 엔진 답의 sha256, 데이터셋 겹침(글자 · 대칭 · 처음 맞은 데이터셋 줄), 결과가 있습니다.

확인하려면 원천 파일 넷을 `sources` 폴더에 내려받고(주소와 파일 sha256은 `sources.txt`), 위에서 지은 검사기로 다음을 돌립니다.

```
mkdir sources
curl -L -o sources/sudoku.txt https://raw.githubusercontent.com/attractivechaos/plb/master/sudoku/sudoku.txt
curl -L -o sources/top95.txt https://norvig.com/top95.txt
curl -L -o sources/hardest.txt https://norvig.com/hardest.txt
curl -L -o sources/diabolical.txt https://raw.githubusercontent.com/grantm/sudoku-exchange-puzzle-bank/master/diabolical.txt
mkdir outside_certs && tar -xzf outside_dataset_726/certificates.tar.gz -C outside_certs
python3 outside_dataset_726/build_boards.py outside_dataset_726/results.tsv sources . > outside_boards.tsv
verify/sudoku-rup-check/target/release/sudoku-rup-check batch outside_boards.tsv outside_certs/proofs
```

- `build_boards.py`는 판 글을 원천에서 꺼내 sha256을 맞대고, 기대 격자 없이(`-`) 검사기에 넘깁니다. 끝 줄이 `TOTAL boards=726 verified=726 open=0 failed=0`이면 726판 모두 증명서가 유일한 해를 정한다는 뜻입니다.
- 검사기는 `VERIFIED` 줄마다 정한 격자를 찍습니다. 그 격자의 sha256(`printf '%s' '<격자>' | sha256sum`)이 그 판의 `answer_sha256`과 같으면, 증명서가 정한 해가 엔진의 답입니다.

## 라이선스

이 묶음은 저장소 맨 위 `LICENSE`(Apache License 2.0)를 따릅니다. 예외로, 검사기 · 증명기 시험 파일(`verify/*/tests/cells.rs`)에 시험 자료로 실은 널리 알려진 판의 판 글과 정답 격자는 이 라이선스에 들지 않습니다(각 판의 공개 출처에서 온 것입니다).

---

# Sudoku 10,000-puzzle result · answer certificates · checker

From the public test split of a public dataset, 10,000 puzzles were drawn by a rule fixed before any result and solved by the reasoning engine. Each puzzle comes with an answer certificate: a proof that the engine's answer is the puzzle's only solution. The certificates were made by a public prover that is given only the puzzle and the answer; they do not show how the engine solved the puzzle. Anyone can re-check every answer from the original dataset and these certificates alone.

## Data and the drawn puzzles

| Item | Value |
|---|---|
| Dataset | `test.csv` of `sapientinc/sudoku-extreme` on Hugging Face (422,786 puzzles · file sha256 `a2fd52aea23d331d5b4ee723c856236e838a9fb9a70e66f4e0e0cf26c338c6a8`) |
| Puzzle name | The data line number in `test.csv` (the line after the header is 1) |
| Drawing rule | For each puzzle number n, key = sha256(`GBR_SUDOKU_EXTREME_TEST_PUBLIC_WHOLE_V1` · 0x00 · n in decimal); ascending key (ties by n); the first 10,000 · fixed before the run |
| Seen in development | 2,621 of the 10,000 puzzles were used during development (all 115 puzzles rated 100 or more are among them); the counts without them are reported too |

## Result

| Count | Puzzles | Solved and verified |
|---|---|---|
| All | 10,000 | 10,000 |
| Not seen in development | 7,379 | 7,379 |
| Rating 0 | 1,410 | 1,410 |
| Rating 1–9 | 2,386 | 2,386 |
| Rating 10–49 | 5,102 | 5,102 |
| Rating 50–99 | 987 | 987 |
| Rating 100 or more | 115 | 115 |

- "Verified": the checker below accepts the puzzle's certificate, and the unique solution the certificate fixes equals the dataset's solution. The certificate was made from the engine's answer. The `answer_sha256` column is the sha256 of the engine's 81-digit answer, and for all 10,000 puzzles it equals the sha256 of the dataset's solution. What can be checked from outside is that the dataset's solution is the puzzle's only solution and that the stated engine answer is that solution. The evidence that the engine produced the answer (the certificates of the solving process) was checked internally, as the next point says.
- Each puzzle was solved once, with no guessing and no backtracking. "No guessing" means the solver never carries on from a candidate assumed to be right. This property was checked internally: the 10,000 certificates that follow the solving process were all checked with the same checker and with drat-trim; they reveal the solving method and are not part of this package. No stored memory was read.
- This is the reasoning engine run directly, not a full-system-path score. For the first 100 puzzles, the full system path gave the same answer, puzzle by puzzle.
- Two checks were made. The checker included here accepted all 10,000 answer certificates (10,000 accepted, 0 refused), and each answer was compared with the dataset's solution: none was wrong. The public prover calls this checker's CNF and unit-propagation code to build its certificates and to check them itself, so this check shares code with the prover. For that reason all 10,000 were also checked with drat-trim, an outside checker we did not write: VERIFIED 10,000 (see "Checker and prover" below).
- The distribution of the 10,000 against the whole test split, by rating band, is in `sudoku_10k_distribution.tsv`.
- The same engine also solved 726 puzzles from outside this dataset (see "Puzzles outside the dataset" below).

## The Sudoku rules are counted from examples (standard 9×9)

The engine does not build the board rules (which cells may not hold the same digit) from the size of the board; it counts them from the answer examples of the dataset's training split (`train.csv`).

- Counting the answers of `train.csv` from the top, the first 81 answers already give rules equal to the standard Sudoku rules (rows, columns and boxes).
- Solving the 10,000 puzzles again with the counted rules gives the same engine output as above, byte for byte; the answers are the same, so the certificates are the same.
- On the public path, the counted rules are checked against the standard rules, and the engine stops instead of solving if they differ.

This section is about the standard 9×9 only. Variant Sudoku is not part of this package.

## Test environment

| Item | Value |
|---|---|
| Machine | One desktop-class PC · Intel Core i5-13500 (14 cores · 20 threads) · 64 GB memory · NVMe SSD |
| GPU | Not used (CPU only) |
| Operating system | Ubuntu 24.04 LTS |
| Solving | Each puzzle once · 10 puzzles at a time · 10,000 puzzles in 1 h 56 min (6,939 s) · at most about 2.1 GB per solver process |
| Building the certificates | The public prover (Rust 1.96) · the 10,000 puzzles in 16 parts at a time: 159 s · 2,129 s of CPU time in all |
| Checking | The public checker (Rust 1.96): the 10,000 in one process, 10.6 s, at most about 90 MB of memory · drat-trim (commit `2e3b2dc0`, gcc 13.3): the 10,000, 16 at a time, 88 s |
| Data | `test.csv` sha256 `a2fd52aea23d331d5b4ee723c856236e838a9fb9a70e66f4e0e0cf26c338c6a8` |

## Files

| File | Content |
|---|---|
| `sudoku_10k_results.tsv` | One line per puzzle: `line` · `line_sha256` (sha256 of that line's bytes) · `rating` · `seen_in_development` · `outcome` · `answer_sha256` (sha256 of the engine's 81-digit answer) · `proof_lemmas` (the number of lemma clauses in that puzzle's certificate) · `correct` · no puzzle text, no grids |
| `sudoku_10k_counts.tsv` | Three counts: all · not seen only · per rating band (with the seen share) |
| `sudoku_10k_distribution.tsv` | Per rating band: puzzles and share in the whole test split; puzzles, share, seen, not seen and solved-and-verified in the 10,000 |
| `sudoku_10k_drat_trim.txt` | The outside check with drat-trim: the sha256 of the certificate archive, the pin (source commit, file sha256, build command), the per-puzzle commands and the counts |
| `certificates/sudoku_10k_certificates.tar.gz` | One answer certificate `proofs/<line>.drup` per puzzle, 10,000 files (4,330,726 bytes unpacked) |
| `verify/sudoku-rup-check/` | The checker's source (Rust, standard library only) |
| `verify/sudoku-rup-prover/` | The public prover's source (Rust, standard library and the checker crate above only) |
| `certificate_samples/` | Three certificates to read without unpacking (103, 1581, 5451; 103 needs no lemma, so its file is empty) |
| `outside_dataset_726/results.tsv` | The 726 puzzles outside the dataset: per puzzle `line` (the certificate's name) · `set` · `source` · `source_line` · `rating` · `puzzle_sha256` · `answer_sha256` (sha256 of the engine's 81-digit answer) · the dataset overlap (`dataset_same_text` · `dataset_same_up_to_symmetry` · `dataset_match`) · `solved` · `checker` · `drat_trim` · no puzzle text, no grids |
| `outside_dataset_726/certificates.tar.gz` | The answer certificates of the 726 puzzles, `proofs/<line>.drup` |
| `outside_dataset_726/build_boards.py` | Takes the puzzle texts from the source files (checked against their sha256) and writes the checker's input |
| `outside_dataset_726/sources.txt` | The four sources' addresses, sizes and sha256, the selection rule, the dataset overlap counts |
| `SHA256SUMS` | sha256 of the files above |

Puzzle text and grids are left out because the dataset card states no license; `line` and `line_sha256` match each puzzle to its original dataset line.

## Certificate form

- Each puzzle uses the standard Sudoku CNF: the variable `81·row + 9·column + digit` (row and column 0–8, digit 1–9; variables 1 to 729); "at least one" and "not two" clauses for every cell, row, column and box; and a unit clause per given digit.
- A certificate is a list of lemma clauses added to that CNF, one clause per line in DIMACS form ending in `0`, additions only (DRUP).
- Each lemma, in order, is checked by unit propagation against the clauses before it (RUP). Once all are accepted and unit propagation fixes all 81 cells, that grid is the unique solution.
- The public prover builds each certificate from two strings only, the puzzle and the answer, by a fixed public rule (described in `verify/sudoku-rup-prover/README.md`). The same puzzle and answer always give the same bytes. No code, order or table of the solving engine is used.
- The 10,000 certificates hold 500,921 lemma clauses in all (267,431 unit clauses · 233,490 two-literal clauses · 0 longer). For 1,068 puzzles unit propagation alone fixes all 81 cells, so their certificates are empty. Lemma clauses per puzzle: 0 for 1,068 puzzles · 1–9 for 1,487 · 10–99 for 6,303 · 100–999 for 1,142.

## Checker and prover

```
curl -L -o test.csv https://huggingface.co/datasets/sapientinc/sudoku-extreme/resolve/main/test.csv
echo "a2fd52aea23d331d5b4ee723c856236e838a9fb9a70e66f4e0e0cf26c338c6a8  test.csv" | sha256sum -c
cargo build --release --manifest-path verify/sudoku-rup-check/Cargo.toml
mkdir certs && tar -xzf certificates/sudoku_10k_certificates.tar.gz -C certs
verify/sudoku-rup-check/target/release/sudoku-rup-check dataset test.csv certs/proofs
```

- The first two lines download the original dataset file and check its sha256 (`test.csv: OK`). All the commands here run in this package's folder.
- It prints one tab-separated line per puzzle, `<line>` · `VERIFIED` · `matches_answer lemmas=<n>` (or `OPEN` · `FAILED <reason>`), and a final `TOTAL boards=… verified=… open=… failed=…`. A final line of `TOTAL boards=10000 verified=10000 open=0 failed=0` means all 10,000 puzzles are verified.
- Checked with the outside checker drat-trim (public source, commit `2e3b2dc0`): VERIFIED 10,000 of 10,000, NOT VERIFIED 0. For each puzzle, `sudoku-rup-check dimacs <puzzle81> <grid81>` (the CNF with a clause that blocks the grid) and `sudoku-rup-check drat <certificate>` (the DRAT text ending in the empty clause) went into `drat-trim`; VERIFIED means that no solution other than the grid exists. The pin, the commands and the counts are in `sudoku_10k_drat_trim.txt`.

To build the certificates again (only the puzzle and the answer go in):

```
cargo build --release --manifest-path verify/sudoku-rup-prover/Cargo.toml
tail -n +2 sudoku_10k_results.tsv | cut -f1 > lines.txt
awk -F, 'NR == FNR {want[$1]; next} FNR > 1 {n = FNR - 1; if (n in want) print n "\t" $2 "\t" $3}' lines.txt test.csv > boards.tsv
mkdir proofs2
verify/sudoku-rup-prover/target/release/sudoku-rup-prover batch boards.tsv proofs2
diff -r certs/proofs proofs2
```

- `boards.tsv` holds one tab-separated line per puzzle: `<line>` · `question` · `answer` (the two columns of that line of `test.csv`). The engine's answer equals the dataset's solution for all 10,000 puzzles (checked by `answer_sha256`), so giving the dataset's solution is giving the same input as the engine's answer.
- A final line of `TOTAL boards=10000 proved=10000 failed=0` and no output from `diff -r` mean that certificates identical to the published ones, byte for byte, were built again. One process takes long (about 35 minutes of CPU time in all), so `boards.tsv` may be cut with `split` and the parts run side by side; the files come out the same.
- `answer_sha256` is the value of `printf '%s' '<answer column>' | sha256sum`.

## Puzzles outside the dataset

With the same engine under the same conditions (each puzzle once), 726 standard 9×9 puzzles were solved; they were taken from four public lists by a rule fixed before the run (sources and rule in `outside_dataset_726/sources.txt`).

| Set | Source | Puzzles | Same as a dataset puzzle | Outside the dataset | Solved and verified |
|---|---|---|---|---|---|
| Named | four widely published hard puzzles + the 20 lines of attractivechaos/plb `sudoku.txt` | 23 | 22 | 1 | 23 |
| Norvig | norvig.com `top95.txt` · `hardest.txt` | 104 | 93 | 11 | 104 |
| Puzzle bank | the first 500 lines of Sudoku Exchange `diabolical.txt` (public domain) | 500 | 0 | 500 | 500 |
| Puzzle bank, top rated | the 100 highest-rated lines of the same file (one already taken left out) | 99 | 0 | 99 | 99 |
| Total | | 726 | 115 | 611 | 726 |

- "Same as a dataset puzzle": the same text as a puzzle of the dataset (`train.csv` · `test.csv`), or the same after a Sudoku symmetry (transposition, the order of the bands and of the stacks, the order of the rows within a band and of the columns within a stack, a relabelling of the digits). No puzzle had the same text; 115 were the same up to symmetry. All the named puzzles but AI Escargot are in the dataset.
- The 611 puzzles outside the dataset: 611 of 611 solved and verified. They come from outside sources, so they are "outside this dataset", not "never seen by anyone".
- "Verified": the public checker accepts the certificate the public prover built from the engine's answer (that answer is the only solution), and drat-trim reports VERIFIED.
- What can be checked from outside is that each certificate fixes a unique solution and that its sha256 equals the sha256 of the stated engine answer (`answer_sha256`). The evidence that the engine produced the answer (the certificates of the solving process) was checked internally, as for the 10,000.
- Puzzle text is left out of this section's table and files (the sources have different terms of use). As an exception, the test files of the public checker and prover (`verify/sudoku-rup-check/tests/cells.rs` · `verify/sudoku-rup-prover/tests/cells.rs`) hold a few widely published puzzles as test data, four of the 726 among them (AI Escargot · Inkala 2012 · Easter Monster · the first line of `top95.txt`; Inkala 2012 with its solution grid). `build_boards.py` takes Inkala 2012, which none of the four source files holds, from the prover's test file.
- `outside_dataset_726/results.tsv` gives, per puzzle, the source file and line, the sha256 of the puzzle text, the sha256 of the engine's answer, the dataset overlap (same text · up to symmetry · the first dataset line matched) and the results.

To check, download the four source files into a folder `sources` (addresses and file sha256 in `sources.txt`) and run, with the checker built above:

```
mkdir sources
curl -L -o sources/sudoku.txt https://raw.githubusercontent.com/attractivechaos/plb/master/sudoku/sudoku.txt
curl -L -o sources/top95.txt https://norvig.com/top95.txt
curl -L -o sources/hardest.txt https://norvig.com/hardest.txt
curl -L -o sources/diabolical.txt https://raw.githubusercontent.com/grantm/sudoku-exchange-puzzle-bank/master/diabolical.txt
mkdir outside_certs && tar -xzf outside_dataset_726/certificates.tar.gz -C outside_certs
python3 outside_dataset_726/build_boards.py outside_dataset_726/results.tsv sources . > outside_boards.tsv
verify/sudoku-rup-check/target/release/sudoku-rup-check batch outside_boards.tsv outside_certs/proofs
```

- `build_boards.py` takes each puzzle text from its source, checks it against its sha256, and hands it to the checker with no expected grid (`-`). A final line of `TOTAL boards=726 verified=726 open=0 failed=0` means that each of the 726 certificates fixes a unique solution.
- The checker prints the grid it fixed on every `VERIFIED` line. If that grid's sha256 (`printf '%s' '<grid>' | sha256sum`) equals the puzzle's `answer_sha256`, the solution the certificate fixes is the engine's answer.

## License

This package is under the repository's top-level `LICENSE` (Apache License 2.0). As an exception, the texts and solution grids of the widely published puzzles kept as test data in the test files of the checker and the prover (`verify/*/tests/cells.rs`) are not covered by this license; they come from their public sources.
