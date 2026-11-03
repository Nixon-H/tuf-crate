// tuf_rgb_probe.c — read-first probe for the discovered RGB command channel
//
// Windows-side RE (see docs/WINDOWS-PLUGIN-DISCOVERY.md §5-6) recovered ASUSOptimization's
// keyboard-lighting path: WMNB 'DEVS' with IIA0=0x00120057 (TUF_RGB_CMD — a firmware-private
// DEVID absent from every earlier inventory) and IIA1 = command value (observed: 0x50, 0x80,
// 0x100, 0x180; sub-codes 4/5/6 stored to a service global). The same WMNB interface is what
// the in-kernel asus-wmi driver uses — i.e. the kernel-sanctioned channel this project builds on.
//
// This probe is READ-ONLY by default: it issues DSTS (query) for the DEVID and logs the answer.
// DEVS (set) probes are gated behind an explicit module parameter and only send the
// "apply current config" command codes observed in the service (0x80), never colour data.
//
// Build:  make -C /lib/modules/$(uname -r)/build M=$PWD modules
// Load:   insmod tuf_rgb_probe.ko            (DSTS probe only, dmesg)
// DEVS:   insmod tuf_rgb_probe.ko probe_devs=1

#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>

MODULE_IMPORT_NS("ASUS_WMI");

/* asus_wmi_evaluate_method is not exported to modules on all kernels; use the WMI bus
 * interface directly if needed. On the target's 7.x kernel it is exported (used by
 * asus-nb-wmi modules). Declare locally to avoid header dependencies: */
int asus_wmi_evaluate_method(u32 method_id, u32 arg0, u32 arg1, u32 *retval);

#define ASUS_WMI_METHODID_DSTS 0x53545344
#define ASUS_WMI_METHODID_DEVS 0x53564544

#define DEVID_TUF_RGB_CMD 0x00120057

static int probe_devs;
module_param(probe_devs, int, 0444);
MODULE_PARM_DESC(probe_devs, "1 = also send the benign DEVS apply command 0x80 (write!)");

static int __init tuf_rgb_probe_init(void)
{
	u32 ret = 0;
	int err;

	pr_info("tuf_rgb_probe: DSTS(0x00120057) ...\n");
	err = asus_wmi_evaluate_method(ASUS_WMI_METHODID_DSTS, DEVID_TUF_RGB_CMD, 0, &ret);
	pr_info("tuf_rgb_probe: DSTS err=%d retval=0x%08x\n", err, ret);
	if (!err && ret != 0xFFFFFFFE)
		pr_info("tuf_rgb_probe: *** DEVID RESPONDS — WMI RGB channel is live ***\n");

	/* other RGB DEVIDs for comparison */
	err = asus_wmi_evaluate_method(ASUS_WMI_METHODID_DSTS, 0x00100066, 0, &ret);
	pr_info("tuf_rgb_probe: DSTS(0x00100066) err=%d retval=0x%08x\n", err, ret);

	if (probe_devs) {
		pr_info("tuf_rgb_probe: DEVS(0x00120057, 0x80) — apply-current command ...\n");
		err = asus_wmi_evaluate_method(ASUS_WMI_METHODID_DEVS, DEVID_TUF_RGB_CMD, 0x80, &ret);
		pr_info("tuf_rgb_probe: DEVS err=%d retval=0x%08x\n", err, ret);
		if (!err && ret != 0xFFFFFFFE)
			pr_info("tuf_rgb_probe: *** DEVS accepted — command channel confirmed ***\n");
	}
	return 0;
}

static void __exit tuf_rgb_probe_exit(void) { }

MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("tuf-crate: read-first probe for WMNB DEVID 0x00120057 (TUF_RGB_CMD)");
module_init(tuf_rgb_probe_init);
module_exit(tuf_rgb_probe_exit);
