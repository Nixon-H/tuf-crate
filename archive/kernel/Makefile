obj-m += tuf_ec.o tuf_rgb.o tuf_rgb_probe.o tuf_rgb_dsts.o

KDIR ?= /lib/modules/$(shell uname -r)/build

all:
	$(MAKE) -C $(KDIR) M=$(CURDIR) modules

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) clean
