// SPDX-License-Identifier: GPL-2.0
/*
 * tuf_ec.c — ASUS TUF Gaming F17 FX706HF keyboard-RGB EC access
 *
 * Scope is intentionally microscopic: this driver exposes EXACTLY four EC
 * bytes (0xA0-0xA3) — the keyboard-lighting colour/mode fields identified by
 * a controlled EC-diff experiment (dynamic=00 00 00 00 vs static-red=00 17 60 00,
 * SHA-256-verified dumps, research/ec/ in the tuf-crate repo).
 *
 * Safety properties ("brick-proof by construction"):
 *  - Writes are only possible to 0xA0-0xA3; every other EC offset is
 *    unreachable through this driver (hard allowlist in code).
 *  - All access goes through the kernel's ec_read()/ec_write(), which
 *    serialize with the ACPI EC driver — no direct port I/O, no racing
 *    with other EC users.
 *  - Every write is read back and verified; a mismatch returns -EIO and
 *    does not report success.
 *  - Worst case is cosmetic: a lighting state that persists until the next
 *    Windows/Armoury-Crate change or EC reset. Fan/thermal EC regions are
 *    never touched. The EC's independent critical-temperature protection
 *    is hardware behaviour outside any software path.
 *
 * Interface: /sys/devices/platform/tuf-ec/ec_a0 .. ec_a3 (hex byte, rw)
 */

#include <linux/acpi.h>
#include <linux/device.h>
#include <linux/module.h>
#include <linux/platform_device.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("tuf-crate project");
MODULE_DESCRIPTION("ASUS TUF FX706HF keyboard RGB EC bytes (verified offsets 0xA0-0xA3 only)");
MODULE_VERSION("0.1");

#define TUF_EC_MIN 0xA0
#define TUF_EC_MAX 0xA3

static ssize_t tuf_ec_byte_show(struct device *dev, struct device_attribute *attr,
				char *buf)
{
	u8 offset, val = 0;
	int ret;

	if (kstrtou8(attr->attr.name + 3, 16, &offset))
		return -EINVAL;
	if (offset < TUF_EC_MIN || offset > TUF_EC_MAX)
		return -EACCES;

	ret = ec_read(offset, &val);
	if (ret)
		return ret;
	return sysfs_emit(buf, "%02x\n", val);
}

static ssize_t tuf_ec_byte_store(struct device *dev, struct device_attribute *attr,
				 const char *buf, size_t count)
{
	u8 offset, val, verify;
	int ret;

	if (kstrtou8(attr->attr.name + 3, 16, &offset))
		return -EINVAL;
	if (offset < TUF_EC_MIN || offset > TUF_EC_MAX)
		return -EACCES;

	if (kstrtou8(buf, 16, &val))
		return -EINVAL;

	ret = ec_write(offset, val);
	if (ret)
		return ret;

	/* Read back: only report success if the EC actually took the value. */
	ret = ec_read(offset, &verify);
	if (ret)
		return ret;
	if (verify != val)
		return -EIO;

	return count;
}

static DEVICE_ATTR(ec_a0, 0644, tuf_ec_byte_show, tuf_ec_byte_store);
static DEVICE_ATTR(ec_a1, 0644, tuf_ec_byte_show, tuf_ec_byte_store);
static DEVICE_ATTR(ec_a2, 0644, tuf_ec_byte_show, tuf_ec_byte_store);
static DEVICE_ATTR(ec_a3, 0644, tuf_ec_byte_show, tuf_ec_byte_store);

static struct attribute *tuf_ec_attrs[] = {
	&dev_attr_ec_a0.attr,
	&dev_attr_ec_a1.attr,
	&dev_attr_ec_a2.attr,
	&dev_attr_ec_a3.attr,
	NULL,
};
ATTRIBUTE_GROUPS(tuf_ec);

static struct platform_device *tuf_ec_pdev;

static struct platform_driver tuf_ec_driver = {
	.driver = {
		.name = "tuf-ec",
		.dev_groups = tuf_ec_groups,
	},
};

static int __init tuf_ec_init(void)
{
	int ret;

	ret = platform_driver_register(&tuf_ec_driver);
	if (ret)
		return ret;

	tuf_ec_pdev = platform_device_register_simple("tuf-ec", -1, NULL, 0);
	if (IS_ERR(tuf_ec_pdev)) {
		platform_driver_unregister(&tuf_ec_driver);
		return PTR_ERR(tuf_ec_pdev);
	}

	pr_info("tuf_ec: FX706HF keyboard RGB EC access ready (offsets 0xA0-0xA3 only)\n");
	return 0;
}

static void __exit tuf_ec_exit(void)
{
	platform_device_unregister(tuf_ec_pdev);
	platform_driver_unregister(&tuf_ec_driver);
	pr_info("tuf_ec: unloaded (EC lighting state persists until changed)\n");
}

module_init(tuf_ec_init);
module_exit(tuf_ec_exit);
