// SPDX-License-Identifier: GPL-2.0
/* tuf_rgb_dsts.c — read-first DSTS/DEVS probe for the TUF RGB DEVIDs.
 *
 * On load it queries DSTS(devid) and prints the answer.  Optional DEVS test:
 *   insmod tuf_rgb_dsts.ko devid=0x00100056 arg1=0x00ff00b4 arg2=0x00000100
 * uses the *correct* WMNB buffer layout (arg0 = DEVID, arg1/arg2 = IIA1/IIA2;
 * method magic 'DEVS' is passed as the WMI method id, never as a buffer dword).
 */
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/init.h>
#include <linux/acpi.h>
#include <linux/wmi.h>

MODULE_IMPORT_NS("ASUS_WMI");

#define ASUS_WMI_MGMT_GUID "97845ED0-4E6D-11DE-8A39-0800200C9A66"
#define ASUS_WMI_METHODID_DSTS 0x53545344
#define ASUS_WMI_METHODID_DEVS 0x53564544

static int devid = 0x00100056;
module_param(devid, int, 0444);
static int arg1;
module_param(arg1, int, 0444);
static int arg2;
module_param(arg2, int, 0444);
static int send_devs;
module_param(send_devs, int, 0444);

struct bios_args {
	u32 arg0;
	u32 arg1;
	u32 arg2;
	u32 arg3;
	u32 arg4;
	u32 arg5;
} __packed;

static int tuf_eval(u32 method, u32 a0, u32 a1, u32 a2, u32 *ret)
{
	struct bios_args args = { .arg0 = a0, .arg1 = a1, .arg2 = a2 };
	struct acpi_buffer input = { (acpi_size)sizeof(args), &args };
	struct acpi_buffer output = { ACPI_ALLOCATE_BUFFER, NULL };
	union acpi_object *obj;
	acpi_status st;
	u32 v = 0;

	st = wmi_evaluate_method(ASUS_WMI_MGMT_GUID, 0, method, &input, &output);
	if (ACPI_FAILURE(st))
		return -EIO;
	obj = output.pointer;
	if (obj && obj->type == ACPI_TYPE_INTEGER)
		v = (u32)obj->integer.value;
	kfree(obj);
	*ret = v;
	return 0;
}

static int __init tuf_rgb_dsts_init(void)
{
	u32 ret = 0;
	int err;

	err = tuf_eval(ASUS_WMI_METHODID_DSTS, devid, arg1, 0, &ret);
	pr_info("tuf_rgb_dsts: DSTS(0x%08x, 0x%08x) err=%d retval=0x%08x\n",
		devid, arg1, err, ret);

	if (send_devs) {
		err = tuf_eval(ASUS_WMI_METHODID_DEVS, devid, arg1, arg2, &ret);
		pr_info("tuf_rgb_dsts: DEVS(0x%08x, 0x%08x, 0x%08x) err=%d retval=0x%08x\n",
			devid, arg1, arg2, err, ret);
	}
	return 0;
}

static void __exit tuf_rgb_dsts_exit(void) { }

MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("tuf-crate: DSTS/DEVS probe for TUF RGB DEVIDs (correct WMNB layout)");
module_init(tuf_rgb_dsts_init);
module_exit(tuf_rgb_dsts_exit);
