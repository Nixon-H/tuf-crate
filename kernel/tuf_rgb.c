// SPDX-License-Identifier: GPL-2.0
/*
 * tuf_rgb.c — ASUS TUF keyboard RGB control via the documented ASUS WMI
 * interface (DEVS method, TUF_KB device IDs), protocol source: G-Helper's
 * public working implementation for TUF laptops (app/AsusACPI.cs).
 *
 * Safety ("brick-proof by construction"):
 *  - The only hardware operation is one WMI DEVS call targeting the
 *    keyboard-lighting device IDs (0x00100056 / 0x0010005a) — the same
 *    kernel channel as every other ASUS attribute. No EC port I/O,
 *    no SMI/GNVS access, no writes outside the lighting command.
 *  - Every value is range-checked before the call (mode <= 0x0f,
 *    colour 0-255, speed <= 0x0f).
 *  - The call is idempotent and reversible (re-send Static with any colour).
 *  - Worst case: the lighting effect changes; EC thermal protection is
 *    independent hardware behaviour.
 *
 * Interface: /sys/devices/platform/tuf-rgb/apply  (write: "mode r g b speed")
 *            read back the last result code ("1" = firmware acknowledged).
 */

#include <linux/acpi.h>
#include <linux/module.h>
#include <linux/platform_device.h>
#include <linux/wmi.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("tuf-crate project");
MODULE_DESCRIPTION("ASUS TUF keyboard RGB via ASUS WMI DEVS (G-Helper protocol)");
MODULE_VERSION("0.1");

/* ASUS WMI: management GUID + DEVS method (same as kernel asus-wmi) */
#define ASUS_WMI_MGMT_GUID "97845ED0-4E6D-11DE-8A39-0800200C9A66"
#define ASUS_WMI_DEVS      0x53564544

/* TUF keyboard RGB device IDs (G-Helper AsusACPI.cs) */
#define TUF_KB   0x00100056
#define TUF_KB2  0x0010005a

/* sub-commands inside the RGB payload */
#define TUF_RGB_SET  0xb4
#define TUF_RGB_SET2 0xb3

/* AuraMode values (G-Helper USB/Aura.cs): Static=0, Breathing=1,
 * ColorCycle=2, Rainbow=3, Star=4, Rain=5, Highlight=6, Laser=7,
 * Ripple=8, Strobe=10, Comet=11, Flash=12 */

struct bios_args {
	u32 arg0; /* method magic: DEVS */
	u32 arg1; /* device id */
	u32 arg2; /* param dword 1: 0xb4 | mode<<8 | r<<16 | g<<24 */
	u32 arg3; /* param dword 2: b | speed<<8 */
	u32 arg4;
} __packed;

static int tuf_rgb_send(u32 devid, u32 a1, u32 a2, u32 *retval)
{
	struct bios_args args = {
		.arg0 = ASUS_WMI_DEVS,
		.arg1 = devid,
		.arg2 = a1,
		.arg3 = a2,
		.arg4 = 0,
	};
	struct acpi_buffer input = { (acpi_size)sizeof(args), &args };
	struct acpi_buffer output = { ACPI_ALLOCATE_BUFFER, NULL };
	union acpi_object *obj;
	acpi_status status;
	u32 result;

	status = wmi_evaluate_method(ASUS_WMI_MGMT_GUID, 0, ASUS_WMI_DEVS,
				     &input, &output);
	if (ACPI_FAILURE(status))
		return -EIO;

	obj = output.pointer;
	if (obj && obj->type == ACPI_TYPE_INTEGER)
		result = (u32)obj->integer.value;
	else
		result = 0;
	kfree(obj);

	if (retval)
		*retval = result;
	return 0;
}

static u32 last_result = 0;

static ssize_t apply_store(struct device *dev, struct device_attribute *attr,
			   const char *buf, size_t count)
{
	unsigned int mode, r, g, b, speed;
	u32 a1, a2, result;
	int ret;

	if (sscanf(buf, "%u %u %u %u %u", &mode, &r, &g, &b, &speed) != 5)
		return -EINVAL;
	if (mode > 0x0f || r > 255 || g > 255 || b > 255 || speed > 0x0f)
		return -EINVAL;

	/* dword packing per G-Helper DeviceSet(TUF_KB, [0xb4, mode, R, G, B, speed]):
	 * buffer bytes: [dev_id(4)] [0xb4 mode R G] [B speed 0 0] */
	a1 = 0xb4u | (mode << 8) | (r << 16) | (g << 24);
	a2 = b | (speed << 8);

	ret = tuf_rgb_send(TUF_KB, a1, a2, &result);
	if (ret)
		return ret;

	if (result != 1) {
		/* G-Helper fallback: TUF_KB2 with 0xb3 then 0xb4 */
		a1 = 0xb3u | (mode << 8) | (r << 16) | (g << 24);
		ret = tuf_rgb_send(TUF_KB2, a1, a2, &result);
		if (ret)
			return ret;
		a1 = 0xb4u | (mode << 8) | (r << 16) | (g << 24);
		ret = tuf_rgb_send(TUF_KB2, a1, a2, &result);
		if (ret)
			return ret;
	}

	last_result = result;
	return count;
}

static ssize_t apply_show(struct device *dev, struct device_attribute *attr,
			  char *buf)
{
	/* firmware "OK" acknowledgement is retval == 1 */
	return sysfs_emit(buf, "%u\n", last_result);
}
static DEVICE_ATTR_RW(apply);

static ssize_t supported_show(struct device *dev, struct device_attribute *attr,
			      char *buf)
{
	/* 1 if the last apply was firmware-acknowledged (retval == 1) */
	return sysfs_emit(buf, "%u\n", last_result == 1 ? 1 : 0);
}
static DEVICE_ATTR_RO(supported);

static struct attribute *tuf_rgb_attrs[] = {
	&dev_attr_apply.attr,
	&dev_attr_supported.attr,
	NULL,
};
ATTRIBUTE_GROUPS(tuf_rgb);

static struct platform_device *tuf_rgb_pdev;

static struct platform_driver tuf_rgb_driver = {
	.driver = {
		.name = "tuf-rgb",
		.dev_groups = tuf_rgb_groups,
	},
};

static int __init tuf_rgb_init(void)
{
	int ret;

	if (!wmi_has_guid(ASUS_WMI_MGMT_GUID)) {
		pr_err("tuf_rgb: ASUS WMI management GUID not present\n");
		return -ENODEV;
	}

	ret = platform_driver_register(&tuf_rgb_driver);
	if (ret)
		return ret;

	tuf_rgb_pdev = platform_device_register_simple("tuf-rgb", -1, NULL, 0);
	if (IS_ERR(tuf_rgb_pdev)) {
		platform_driver_unregister(&tuf_rgb_driver);
		return PTR_ERR(tuf_rgb_pdev);
	}

	pr_info("tuf_rgb: TUF keyboard RGB via ASUS WMI DEVS ready (TUF_KB 0x00100056, TUF_KB2 0x0010005a)\n");
	return 0;
}

static void __exit tuf_rgb_exit(void)
{
	platform_device_unregister(tuf_rgb_pdev);
	platform_driver_unregister(&tuf_rgb_driver);
}

module_init(tuf_rgb_init);
module_exit(tuf_rgb_exit);
