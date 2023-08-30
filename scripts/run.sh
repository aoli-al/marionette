for i in `seq 1 20`; do
    echo "${@:1}"
	python3 ./scripts/run.py "${@:1}"
done
