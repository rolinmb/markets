import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages
from matplotlib.table import Table
import pandas as pd
import os

CSVDIR = 'csv_out'
IMGDIR = 'img_out'

def add_financials_table(fdf, ax):
    ax.axis('tight')
    ax.axis('off')
    ftable = Table(ax, bbox=[0, 0, 1, 1])
    for (j, label) in enumerate(fdf.columns):
        ftable.add_cell(0, j, width=0.1, height=0.05, text=label, loc='center', facecolor='lightgrey')
    for i, row in enumerate(fdf.itertuples(), start=1):
        for j, value in enumerate(row[1:], start=0):
            ftable.add_cell(i, j, width=0.1, height=0.05, text=value, loc='center')
    ax.add_table(ftable)

def generate_pdf(pdf_name):
    with PdfPages(pdf_name) as pdf:
        for csv_file in os.listdir(CSVDIR):
            csv_path = os.path.join(CSVDIR, csv_file)
            csv_type = csv_file.split('_')[1]
            if csv_type == 'fv':
                try:
                    fdf = pd.read_csv(csv_path)
                    plt.figure(figsize=[12, 8], dpi=100)
                    ax = plt.subplot(111)
                    add_financials_table(fdf, ax)
                    pdf.savefig(bbox_inches='tight')
                    plt.close()
                except Exception as e:
                    print(f'\ngenerate_pdf :: ERROR -> A problem occured while processing financial statement data file {csv_file}:\n\n{e}\n\ngenerate_pdf :: continuing pdf generation without financial statement data table\n\n')
                continue
            else:
                print(f'\ngenerate_pdf :: Skipping csv file {csv_file}')
        print(f'\ngenerate_pdf :: Added financial statement data from {csv_file} to pdf')
        for img_file in os.listdir(IMGDIR):
            img_path = os.path.join(IMGDIR, img_file)
            continue # TODO: Iterate over option chain and time series images to generate PDF
    print(f'\ngenerate_pdf :: Successfully created pdf report as {pdf_name}')

if __name__ == '__main__':
    generate_pdf('pdf_out/test.pdf')