use itertools::Itertools;
use x86_64::{
    structures::paging::{PageSize, PageTable, PageTableFlags, Size1GiB, Size2MiB, Size4KiB},
    VirtAddr,
};

use crate::GenIter;

use super::{get_page_table, physical_memory_offset};

pub fn explore_page_ranges() -> impl Iterator<Item = (VirtAddr, VirtAddr, PageTableFlags)> {
    let my_genm = GenIter::from(static || unsafe {
        let offset = physical_memory_offset();
        let mut table = get_page_table();
        let table = table.level_4_table();

        for (i4, x4) in table.iter().enumerate() {
            if !x4.flags().contains(PageTableFlags::PRESENT) {
                continue;
            }
            let start4 = VirtAddr::new(i4 as u64 * 512 * Size1GiB::SIZE);
            for (i3, x3) in (&*(offset + x4.addr().as_u64()).as_ptr() as &PageTable)
                .iter()
                .enumerate()
            {
                if !x3.flags().contains(PageTableFlags::PRESENT) {
                    continue;
                }
                let start3 = start4 + i3 * Size1GiB::SIZE as usize;
                if x3.flags().contains(PageTableFlags::HUGE_PAGE) {
                    yield (start3, start3 + Size1GiB::SIZE as usize, x3.flags());
                    continue;
                }

                for (i2, x2) in (&*(offset + x3.addr().as_u64()).as_ptr() as &PageTable)
                    .iter()
                    .enumerate()
                {
                    if !x2.flags().contains(PageTableFlags::PRESENT) {
                        continue;
                    }
                    let start2 = start3 + i2 * Size2MiB::SIZE as usize;
                    if x2.flags().contains(PageTableFlags::HUGE_PAGE) {
                        yield (start2, start2 + Size2MiB::SIZE as usize, x2.flags());
                        continue;
                    }

                    for (i1, x1) in (&*(offset + x2.addr().as_u64()).as_ptr() as &PageTable)
                        .iter()
                        .enumerate()
                    {
                        if !x1.flags().contains(PageTableFlags::PRESENT) {
                            continue;
                        }
                        let start1 = start2 + i1 * Size4KiB::SIZE as usize;
                        yield (start1, start1 + Size4KiB::SIZE as usize, x1.flags());
                    }
                }
            }
        }
    });
    let removed_fields =
        PageTableFlags::ACCESSED | PageTableFlags::DIRTY | PageTableFlags::HUGE_PAGE;
    my_genm
        .map(move |(f, t, flags)| (f, t, flags - removed_fields))
        .coalesce(|(afrom, ato, aflags), (bfrom, bto, bflags)| {
            if ato == bfrom && aflags == bflags {
                Ok((afrom, bto, aflags))
            } else {
                Err(((afrom, ato, aflags), (bfrom, bto, bflags)))
            }
        })
}
