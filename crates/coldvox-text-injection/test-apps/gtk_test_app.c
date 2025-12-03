#include <gtk/gtk.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

// Callback function to handle text changes in the GtkEntry
static void on_text_changed(GtkEditable *editable, gpointer user_data) {
    const gchar *text = gtk_entry_get_text(GTK_ENTRY(editable));
    char* filepath = (char*)user_data;

    FILE *f = fopen(filepath, "w");
    if (f == NULL) {
        perror("Error opening file for writing");
        return;
    }
    fprintf(f, "%s", text);
    fclose(f);
}

int main(int argc, char *argv[]) {
    gtk_init(&argc, &argv);

    if (argc < 2) {
        fprintf(stderr, "Usage: %s <output_file_path>\\n", argv[0]);
        return 1;
    }
    char* output_filepath = argv[1];

    GtkWidget *window = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(window), "GTK Test App");
    gtk_window_set_default_size(GTK_WINDOW(window), 200, 50);
    g_signal_connect(window, "destroy", G_CALLBACK(gtk_main_quit), NULL);

    GtkWidget *entry = gtk_entry_new();
    gtk_container_add(GTK_CONTAINER(window), entry);

    g_signal_connect(G_OBJECT(entry), "changed", G_CALLBACK(on_text_changed), output_filepath);

    gtk_widget_show_all(window);
    gtk_widget_grab_focus(entry);

    gtk_main();

    return 0;
}
