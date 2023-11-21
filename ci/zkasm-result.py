import os
import csv
import sys


tests_dir = 'cranelift/zkasm_data/spectest/i64'
generated_dir = 'cranelift/zkasm_data/spectest/i64/generated'
state_csv_path = 'cranelift/codegen/src/isa/zkasm/docs/state.csv'


def check_compilation_status():
    status_map = {}
    for file in os.listdir(tests_dir):
        if not file.endswith('.wat'):
            continue
        test_name = os.path.splitext(file)[0]
        zkasm_file = f'{test_name}.zkasm'
        status_map[test_name] = 'compilation success' if zkasm_file in os.listdir(generated_dir) else 'compilation failed'
    return status_map


def update_status_from_stdin(status_map):
    for line in sys.stdin:
        if "--> fail" in line or "--> pass" in line:
            _, _, test_path = line.partition(' ')
            test_name, _ = os.path.splitext(os.path.basename(test_path))
            status_map[test_name] = 'pass' if 'pass' in line else 'runtime error'


def write_csv(status_map):
    with open(state_csv_path, 'w', newline='') as csvfile:
        csvwriter = csv.writer(csvfile)
        csvwriter.writerow(['Test', 'Status'])
        status_list = sorted(status_map.items())
        csvwriter.writerows(status_list)
        csvwriter.writerow(['Total Passed', sum(1 for status in status_map.values() if status == 'pass')])
        csvwriter.writerow(['Amount of Tests', len(status_map)])


def assert_with_csv(status_map):
    with open(state_csv_path, newline='') as csvfile:
        csvreader = csv.reader(csvfile)
        csv_dict = {}
        for row in csvreader:
            if row[0] in ["Test", "Total Passed", "Amount of Tests"]:
                continue
            csv_dict[row[0]] = row[1]
        if csv_dict != status_map:
            print(f"dict diff = {csv_dict ^ status_map}")
            return 1
    return 0


def main():
    status_map = check_compilation_status()
    update_status_from_stdin(status_map)
    if '--update' in sys.argv:
        write_csv(status_map)
    else:
        if assert_with_csv(status_map) != 0:
            sys.exit(1)
    sys.exit(0)


if __name__ == "__main__":
    main()
