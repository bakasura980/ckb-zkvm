report-bench.txt:
	echo -n > $@
	echo -n "os   : " >> $@
	uname -a >> $@
	echo -n "arch : " >> $@
	uname -m >> $@
	echo -n "cpu  : " >> $@
	cat /proc/cpuinfo | grep name | cut -d: -f2 | cut -c2- | uniq >> $@
	echo "--------------------------------------------------------------------------------" >> $@
	make -C programs report >> $@
	echo "--------------------------------------------------------------------------------" >> $@
	cargo bench >> $@
