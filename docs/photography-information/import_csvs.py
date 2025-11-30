#!/usr/bin/env python3
"""
ShootProof CSV Importer for Photography Database

Imports contacts and orders from ShootProof CSV exports into SurrealDB.
"""

import csv
import argparse
from datetime import datetime
from pathlib import Path
from surrealdb import Surreal

DB_URL = "ws://127.0.0.1:8000/rpc"
DB_NS = "photography"
DB_NAME = "ops"
DB_USER = "root"
DB_PASS = "root"


def import_contacts(db, csv_path: str, dry_run: bool = False):
    """Import contacts CSV into family records."""

    print(f"\n{'[DRY RUN] ' if dry_run else ''}Importing contacts from: {csv_path}")

    created = 0
    updated = 0
    skipped = 0

    with open(csv_path, 'r', encoding='utf-8-sig') as f:
        reader = csv.DictReader(f)

        for row in reader:
            # Extract fields
            contact_id = row.get('Contact ID', '').strip()
            first_name = row.get('First Name', '').strip()
            last_name = row.get('Last Name', '').strip()
            email = row.get('Email', '').strip()
            phone = row.get('Phone', '').strip()
            galleries = row.get('Galleries', '').strip()
            created_at = row.get('Created', '').strip()

            if not last_name:
                skipped += 1
                continue

            # Family ID is lowercase last name
            family_id = last_name.lower().replace(' ', '_').replace("'", "")

            # Check if family exists
            existing = db.query(
                "SELECT * FROM family WHERE id = type::thing('family', $id)",
                {"id": family_id}
            )

            # Build full name
            full_name = f"{first_name} {last_name}".strip() if first_name else last_name

            family_data = {
                "name": full_name,  # Required field
                "last_name": last_name,
                "shootproof_contact_id": int(contact_id) if contact_id.isdigit() else None,
            }

            # Only set email if we have one
            if email:
                family_data["delivery_email"] = email
            if phone:
                family_data["phone"] = phone
            if galleries:
                family_data["galleries"] = [g.strip() for g in galleries.split(',')]

            # surrealdb Python client returns [] when empty, or list of records when found
            has_existing = existing and len(existing) > 0

            if has_existing:
                # Update existing
                if not dry_run:
                    db.query(
                        """UPDATE type::thing('family', $id) MERGE $data""",
                        {"id": family_id, "data": family_data}
                    )
                updated += 1
                print(f"  Updated: {last_name}")
            else:
                # Create new
                if not dry_run:
                    db.query(
                        """CREATE type::thing('family', $id) CONTENT $data""",
                        {"id": family_id, "data": family_data}
                    )
                created += 1
                print(f"  Created: {last_name}")

    print(f"\nContacts summary: {created} created, {updated} updated, {skipped} skipped")
    return created, updated, skipped


def import_orders(db, csv_path: str, dry_run: bool = False):
    """Import orders CSV into order records."""

    print(f"\n{'[DRY RUN] ' if dry_run else ''}Importing orders from: {csv_path}")

    created = 0
    skipped = 0
    families_not_found = set()

    with open(csv_path, 'r', encoding='utf-8-sig') as f:
        reader = csv.DictReader(f)

        for row in reader:
            order_id = row.get('Order ID', '').strip()
            order_date = row.get('Order Date', '').strip()
            gallery = row.get('Gallery', '').strip()
            customer_name = row.get('Customer Name', '').strip()
            customer_email = row.get('Customer Email', '').strip()
            total_sales = row.get('Total Sales', '0').strip().replace(',', '')
            profit = row.get('Profit', '0').strip().replace(',', '')
            items_ordered = row.get('Items Ordered', '').strip()

            if not order_id or not gallery:
                skipped += 1
                continue

            # Extract last name from gallery (usually "FirstName LastName" or just "LastName")
            gallery_parts = gallery.split()
            last_name = gallery_parts[-1] if gallery_parts else gallery
            family_id = last_name.lower().replace(' ', '_').replace("'", "")

            # Check if family exists
            existing_family = db.query(
                "SELECT * FROM family WHERE id = type::thing('family', $id)",
                {"id": family_id}
            )

            # surrealdb Python client returns [] when empty, or list of records when found
            has_family = existing_family and len(existing_family) > 0

            if not has_family:
                families_not_found.add(last_name)
                # Create the family record
                if not dry_run:
                    db.query(
                        """CREATE type::thing('family', $id) CONTENT {
                            name: $name,
                            last_name: $last_name,
                            delivery_email: $email
                        }""",
                        {"id": family_id, "name": gallery, "last_name": last_name, "email": customer_email}
                    )

            # Parse date
            try:
                if ',' in order_date:  # "Jan 2, 2025" format
                    parsed_date = datetime.strptime(order_date, "%b %d, %Y")
                else:
                    parsed_date = datetime.strptime(order_date, "%Y-%m-%d")
                order_date_iso = parsed_date.isoformat()
            except:
                order_date_iso = order_date

            # Parse amounts
            try:
                total = float(total_sales) if total_sales else 0.0
            except:
                total = 0.0
            try:
                net_profit = float(profit) if profit else 0.0
            except:
                net_profit = 0.0

            # Determine if this is a $0 order (comp/correction)
            is_comp = total == 0.0

            # Count items
            item_count = len(items_ordered.split('\n')) if items_ordered else 0

            # Check if order already exists
            existing_order = db.query(
                "SELECT * FROM order WHERE shootproof_order_id = $order_id",
                {"order_id": int(order_id)}
            )

            # surrealdb Python client returns [] when empty, or list of records when found
            has_order = existing_order and len(existing_order) > 0

            if has_order:
                skipped += 1
                continue

            order_data = {
                "shootproof_order_id": int(order_id),
                "order_date": order_date_iso,
                "gallery_name": gallery,
                "customer_name": customer_name,
                "customer_email": customer_email,
                "total_sales": total,
                "profit": net_profit,
                "is_comp": is_comp,
                "item_count": item_count,
                "items_raw": items_ordered[:500] if items_ordered else None,  # Truncate for storage
            }

            if not dry_run:
                # Create order
                result = db.query(
                    "CREATE order CONTENT $data",
                    {"data": order_data}
                )

                # Link order to family
                if result and len(result) > 0 and result[0].get('result'):
                    order_record_id = result[0]['result'][0]['id']
                    db.query(
                        """RELATE type::thing('family', $family_id)->ordered->$order_id SET
                            amount = $amount,
                            order_date = $date""",
                        {
                            "family_id": family_id,
                            "order_id": order_record_id,
                            "amount": total,
                            "date": order_date_iso
                        }
                    )

            created += 1
            if total > 0:
                print(f"  Order #{order_id}: {gallery} - ${total:.2f}")
            else:
                print(f"  Order #{order_id}: {gallery} - $0 (comp)")

    print(f"\nOrders summary: {created} created, {skipped} skipped")
    if families_not_found:
        print(f"Created {len(families_not_found)} new families from orders: {', '.join(sorted(families_not_found)[:10])}{'...' if len(families_not_found) > 10 else ''}")

    return created, skipped


def main():
    parser = argparse.ArgumentParser(description='Import ShootProof CSVs into photography database')
    parser.add_argument('--contacts', type=str, help='Path to contacts CSV')
    parser.add_argument('--orders', type=str, help='Path to orders CSV')
    parser.add_argument('--dry-run', action='store_true', help='Preview without making changes')
    parser.add_argument('--all-2025', action='store_true', help='Import all 2025 data from standard paths')

    args = parser.parse_args()

    # Standard paths
    base_path = Path('/Users/samuelatagana/Projects/LegacyMind/photography-information')

    if args.all_2025:
        args.contacts = str(base_path / 'contacts-2025-11-26.csv')
        args.orders = str(base_path / '2025' / 'orders-from-2025-01-01-to-2025-12-31.csv')

    if not args.contacts and not args.orders:
        parser.print_help()
        return

    # Connect to database
    db = Surreal(DB_URL)
    db.signin({"username": DB_USER, "password": DB_PASS})
    db.use(DB_NS, DB_NAME)

    print(f"Connected to {DB_NS}/{DB_NAME}")

    try:
        if args.contacts:
            import_contacts(db, args.contacts, args.dry_run)

        if args.orders:
            import_orders(db, args.orders, args.dry_run)

        print("\n✅ Import complete!")

    finally:
        db.close()


if __name__ == '__main__':
    main()
